//! Humidity-aware ventilation helpers: sensor freshness, psychrometric
//! aggregation of the configured MQTT sensors, and the moisture/energy decision
//! primitives used by `extreme_heat.rs`.

use crate::app_state::AppStateInner;
use crate::cwl_data::VentLevel;
use crate::psychro;

/// A reading is stale after this long without an update (battery sensors report
/// slowly; 30 min tolerates a missed cycle).
pub const SENSOR_STALE_MS: u32 = 30 * 60 * 1000;

/// Indoor RH (%) at/above which moisture protection considers ventilating.
pub const RH_PROTECT: f32 = 65.0;
/// Indoor RH (%) at/above which protection goes straight to Party.
pub const RH_PROTECT_HIGH: f32 = 75.0;
/// Hysteresis (%): once active, protection only releases below `RH_PROTECT - this`.
pub const RH_HYSTERESIS: f32 = 5.0;
/// Outdoor air must be at least this much drier (g/m³ absolute) for ventilating
/// to actually lower indoor humidity.
pub const AH_MARGIN: f32 = 0.5;
/// Energy bands on Δh = h_out − h_in (kJ/kg): above `H_OFF` → Off, below `H_PARTY` → Party.
pub const H_OFF: f32 = 2.0;
pub const H_PARTY: f32 = -4.0;

pub fn is_fresh(updated_ms: u32, now_ms: u32) -> bool {
    updated_ms != 0 && now_ms.wrapping_sub(updated_ms) < SENSOR_STALE_MS
}

/// Derived psychrometric state of one air mass.
#[derive(Debug, Clone, Copy)]
pub struct AirState {
    pub rh: f32, // relative humidity %
    pub ah: f32, // absolute humidity g/m³
    pub h: f32,  // specific enthalpy kJ/kg
}

/// Decision inputs, available only when there is a fresh outdoor reading and at
/// least one fresh indoor reading.
pub struct HumidityInputs {
    pub indoor: AirState,
    pub outdoor: AirState,
}

/// Build decision inputs from app state. Sensors that report humidity without a
/// temperature fall back to the supplied air-stream temperatures (exhaust inlet
/// indoors, supply inlet outdoors).
pub fn inputs(
    st: &AppStateInner,
    now_ms: u32,
    fallback_indoor_temp: f32,
    fallback_outdoor_temp: f32,
) -> Option<HumidityInputs> {
    let p = st.ambient_pressure_kpa;

    let out = st.humidity_outside?;
    if !is_fresh(out.updated_ms, now_ms) {
        return None;
    }
    let out_t = out.temperature.unwrap_or(fallback_outdoor_temp);
    let outdoor = AirState {
        rh: out.humidity,
        ah: psychro::absolute_humidity(out_t, out.humidity),
        h: psychro::enthalpy(out_t, out.humidity, p),
    };

    // Indoor: worst-case room — keep the highest-absolute-humidity sensor's
    // AH/enthalpy, and the highest RH across all fresh sensors as the trigger.
    let mut best: Option<AirState> = None;
    let mut rh_max = 0.0_f32;
    for s in st.humidity_inside.values() {
        if !is_fresh(s.updated_ms, now_ms) {
            continue;
        }
        let t = s.temperature.unwrap_or(fallback_indoor_temp);
        let ah = psychro::absolute_humidity(t, s.humidity);
        let h = psychro::enthalpy(t, s.humidity, p);
        if s.humidity > rh_max {
            rh_max = s.humidity;
        }
        let replace = match best {
            Some(b) => ah > b.ah,
            None => true,
        };
        if replace {
            best = Some(AirState { rh: s.humidity, ah, h });
        }
    }
    let mut indoor = best?;
    indoor.rh = rh_max.max(indoor.rh);
    Some(HumidityInputs { indoor, outdoor })
}

/// Energy-based level from the enthalpy delta `Δh = h_out − h_in`, generalizing
/// the temperature bands. Returns the level and whether it is cooling-driven.
pub fn enthalpy_level(inp: &HumidityInputs) -> (VentLevel, bool) {
    let dh = inp.outdoor.h - inp.indoor.h;
    if dh > H_OFF {
        (VentLevel::Off, false)
    } else if dh > 0.0 {
        (VentLevel::Reduced, false)
    } else if dh >= H_PARTY {
        (VentLevel::Normal, true)
    } else {
        (VentLevel::Party, true)
    }
}

/// Moisture-protection target level, or `None` if protection should not act.
/// Only ventilates when the outdoor air is genuinely drier (so it dehumidifies);
/// `was_active` applies turn-off hysteresis.
pub fn protection_level(inp: &HumidityInputs, was_active: bool) -> Option<VentLevel> {
    let drier_outside = inp.outdoor.ah < inp.indoor.ah - AH_MARGIN;
    if !drier_outside {
        return None;
    }
    let on_threshold = if was_active { RH_PROTECT - RH_HYSTERESIS } else { RH_PROTECT };
    if inp.indoor.rh < on_threshold {
        return None;
    }
    // Party when very humid or when it also cools; otherwise Normal so we don't
    // import a lot of hot air purely to dehumidify.
    let cooling = inp.outdoor.h < inp.indoor.h;
    if inp.indoor.rh >= RH_PROTECT_HIGH || cooling {
        Some(VentLevel::Party)
    } else {
        Some(VentLevel::Normal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn air(rh: f32, ah: f32, h: f32) -> AirState {
        AirState { rh, ah, h }
    }

    #[test]
    fn muggy_cooler_outdoor_does_not_ventilate() {
        // Outdoor higher energy (humid) despite being cooler → hold down.
        let inp = HumidityInputs { indoor: air(55.0, 11.0, 55.0), outdoor: air(85.0, 13.0, 58.0) };
        let (lvl, cooling) = enthalpy_level(&inp);
        assert_eq!(lvl, VentLevel::Off);
        assert!(!cooling);
    }

    #[test]
    fn warm_but_drier_outdoor_assists_cooling() {
        // Outdoor lower energy (dry) → ventilate, cooling.
        let inp = HumidityInputs { indoor: air(60.0, 13.0, 60.0), outdoor: air(30.0, 9.0, 54.0) };
        let (lvl, cooling) = enthalpy_level(&inp);
        assert_eq!(lvl, VentLevel::Party);
        assert!(cooling);
    }

    #[test]
    fn protection_acts_when_humid_and_outside_drier() {
        let inp = HumidityInputs { indoor: air(70.0, 12.0, 58.0), outdoor: air(40.0, 9.0, 54.0) };
        assert!(protection_level(&inp, false).is_some());
    }

    #[test]
    fn protection_skips_when_outside_not_drier() {
        // Outdoor absolute humidity within the margin → ventilating wouldn't dry.
        let inp = HumidityInputs { indoor: air(70.0, 12.0, 58.0), outdoor: air(72.0, 11.8, 56.0) };
        assert!(protection_level(&inp, false).is_none());
    }

    #[test]
    fn protection_hysteresis_holds_below_threshold_when_active() {
        // RH 62 (< RH_PROTECT 65) but already active → stays on (>= 60); drier outside.
        let inp = HumidityInputs { indoor: air(62.0, 12.0, 58.0), outdoor: air(40.0, 9.0, 54.0) };
        assert!(protection_level(&inp, true).is_some());
        assert!(protection_level(&inp, false).is_none()); // not yet active → don't trigger
    }
}
