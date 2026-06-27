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
/// Hysteresis around Δh = 0 for the hold-down↔cooling choice. Enter cooling only
/// once outdoor air is at least `-H_COOL_ON` kJ/kg lower-energy than indoor;
/// release back to hold-down only once it is no longer lower-energy (above
/// `H_COOL_OFF`). Between the two the current choice is held, so a Δh wandering
/// across zero within sensor noise (a DHT's ±2–5 % RH is already ~±1–2 kJ/kg)
/// neither flips the level nor the displayed reason.
pub const H_COOL_ON: f32 = -1.0;
pub const H_COOL_OFF: f32 = 0.0;

pub fn is_fresh(updated_ms: u32, now_ms: u32) -> bool {
    updated_ms != 0 && now_ms.wrapping_sub(updated_ms) < SENSOR_STALE_MS
}

/// Plausible ambient-temperature window (°C). Readings outside it (a dead sensor
/// reporting 0, a probe in direct sun, a decode glitch) are ignored as
/// temperature candidates so one bad value can't drag the aggregate to an extreme
/// — important because the aggregate takes the *lowest* temperature.
pub const TEMP_PLAUSIBLE_MIN: f32 = -30.0;
pub const TEMP_PLAUSIBLE_MAX: f32 = 55.0;

fn plausible_temp(t: f32) -> bool {
    t.is_finite() && (TEMP_PLAUSIBLE_MIN..=TEMP_PLAUSIBLE_MAX).contains(&t)
}

/// Derived psychrometric state of one air mass (indoor or outdoor), aggregated
/// across that side's sensors: the **lowest** temperature and the **highest**
/// humidity present, combined into one conservative state.
#[derive(Debug, Clone, Copy)]
pub struct AirState {
    pub temp: f32, // aggregate (lowest) temperature °C
    pub rh: f32,   // aggregate (highest) relative humidity %
    pub ah: f32,   // absolute humidity g/m³ at `temp`
    pub h: f32,    // specific enthalpy kJ/kg at `temp`
}

/// Decision inputs, available only when each side has at least one fresh sensor.
pub struct HumidityInputs {
    pub indoor: AirState,
    pub outdoor: AirState,
}

/// Build decision inputs from app state.
///
/// Each side is aggregated across all its fresh sensors as the **lowest
/// temperature** and the **highest humidity** measured, so multiple sensors of
/// either kind can be configured and the worst-case-for-the-decision values win.
/// The air-stream inlet temperatures (supply ID 80 outdoors, exhaust ID 82
/// indoors) join the temperature pool while the fans run, since they then read
/// the true air being moved — which is what lets a misplaced "outdoor" humidity
/// sensor (e.g. one sitting in a warm garden house) contribute its moisture
/// without its inflated temperature skewing the decision.
pub fn inputs(
    st: &AppStateInner,
    now_ms: u32,
    fallback_indoor_temp: f32,
    fallback_outdoor_temp: f32,
) -> Option<HumidityInputs> {
    let p = st.ambient_pressure_kpa;
    let fans_running = st.cwl_data.ventilation_level > 0;
    let outdoor = aggregate_side(
        &st.humidity_outside, now_ms, fallback_outdoor_temp, fans_running, p,
    )?;
    let indoor = aggregate_side(
        &st.humidity_inside, now_ms, fallback_indoor_temp, fans_running, p,
    )?;
    Some(HumidityInputs { indoor, outdoor })
}

/// Aggregate one side's fresh sensors into a single [`AirState`]: lowest
/// temperature, highest humidity, with the moisture content re-expressed at the
/// chosen temperature. `inlet_temp` is the air-stream inlet (ID 80/82); it joins
/// the temperature pool when `fans_running`, and is the last-resort temperature
/// when no sensor reports one. Returns `None` if no fresh sensor is present.
fn aggregate_side(
    sensors: &std::collections::HashMap<String, crate::app_state::HumiditySample>,
    now_ms: u32,
    inlet_temp: f32,
    fans_running: bool,
    p: f32,
) -> Option<AirState> {
    let mut temp_min: Option<f32> = None; // lowest plausible temperature
    let mut rh_max = 0.0_f32; // highest humidity (the protection/display value)
    // Moisture content (as vapour pressure) of the wettest sensor, with the
    // temperature it was measured at so it can be re-expressed at `temp_min`.
    let mut wettest: Option<(f32 /*sensor_t*/, f32 /*rh*/, f32 /*ah*/)> = None;

    for s in sensors.values() {
        if !is_fresh(s.updated_ms, now_ms) {
            continue;
        }
        // A temperature-only sensor (e.g. an outdoor thermometer) still feeds the
        // temperature pool; a humidity-only sensor still feeds the moisture pool.
        if let Some(t) = s.temperature {
            if plausible_temp(t) {
                temp_min = Some(temp_min.map_or(t, |m| m.min(t)));
            }
        }
        if let Some(rh) = s.humidity {
            if rh > rh_max {
                rh_max = rh;
            }
            let moist_t = s.temperature.unwrap_or(inlet_temp);
            let ah = psychro::absolute_humidity(moist_t, rh);
            if wettest.map_or(true, |(_, _, b_ah)| ah > b_ah) {
                wettest = Some((moist_t, rh, ah));
            }
        }
    }

    let (moist_t, moist_rh, _) = wettest?; // None ⇒ no fresh sensor on this side

    // The inlet reads the real moving air only while the fans run; otherwise it
    // holds stale duct air, so it only serves as a last-resort temperature.
    if fans_running && plausible_temp(inlet_temp) {
        temp_min = Some(temp_min.map_or(inlet_temp, |m| m.min(inlet_temp)));
    }
    // Need a trustworthy temperature to place the moisture at. If no sensor
    // reported one and the inlet fallback is implausible (e.g. the boot default
    // before the unit is connected), don't fabricate an air state from it.
    let temp = match temp_min {
        Some(t) => t,
        None if plausible_temp(inlet_temp) => inlet_temp,
        None => return None,
    };

    let mut air = air_at_temp(moist_t, moist_rh, temp, p);
    air.rh = rh_max; // report/trigger on the highest measured humidity
    Some(air)
}

/// Re-express a reading taken at `sensor_t` (°C) / `rh` (%) as the same air mass
/// would be at `air_t` (°C), preserving its moisture content (vapour pressure is
/// conserved when air merely changes temperature). When `air_t == sensor_t` this
/// is the identity; if `air_t` is below the dew point the recomputed RH saturates
/// at 100 %.
fn air_at_temp(sensor_t: f32, rh: f32, air_t: f32, p: f32) -> AirState {
    let pv = psychro::vapor_kpa(sensor_t, rh);
    let air_rh = (pv / psychro::saturation_kpa(air_t) * 100.0).clamp(0.0, 100.0);
    AirState {
        temp: air_t,
        rh: air_rh,
        ah: psychro::absolute_humidity(air_t, air_rh),
        h: psychro::enthalpy(air_t, air_rh, p),
    }
}

/// Energy-based level from the enthalpy delta `Δh = h_out − h_in`, generalizing
/// the temperature bands. Returns the level and whether it is cooling-driven.
///
/// `was_cooling` carries the previous cooling state so the hold-down↔cooling
/// choice is hysteretic around `Δh = 0` ([`H_COOL_ON`]/[`H_COOL_OFF`]): a Δh
/// merely crossing zero by sensor noise keeps the current choice instead of
/// flipping the level (and the displayed reason). The Off and Party bands keep
/// their existing ±margins and are unaffected.
pub fn enthalpy_level(inp: &HumidityInputs, was_cooling: bool) -> (VentLevel, bool) {
    let dh = inp.outdoor.h - inp.indoor.h;
    if dh > H_OFF {
        (VentLevel::Off, false)
    } else if dh < H_PARTY {
        (VentLevel::Party, true)
    } else {
        // Neutral band [H_COOL_ON, H_COOL_OFF]: hold the current choice. From
        // hold-down, only enter cooling below H_COOL_ON; from cooling, only
        // release above H_COOL_OFF.
        let cooling = if was_cooling { dh < H_COOL_OFF } else { dh < H_COOL_ON };
        if cooling {
            (VentLevel::Normal, true)
        } else {
            (VentLevel::Reduced, false)
        }
    }
}

/// Whether the mode is *holding* hold-down (Reduced) through the neutral band
/// even though the bare `Δh ≤ 0` rule would have switched to cooling — i.e. the
/// hysteresis, not a genuine "reduce" decision, is what's keeping the level. Used
/// to explain a withheld switch in the UI. `current_level` is the floored level
/// the mode currently holds (Off is floored to Reduced); `dh = h_out − h_in`.
pub fn deadband_holding(dh: f32, current_level: u8) -> bool {
    current_level <= VentLevel::Reduced as u8 && dh > H_COOL_ON && dh <= H_COOL_OFF
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

    use crate::app_state::HumiditySample;
    use std::collections::HashMap;

    fn air(rh: f32, ah: f32, h: f32) -> AirState {
        AirState { temp: 25.0, rh, ah, h }
    }

    fn sample(humidity: Option<f32>, temperature: Option<f32>) -> HumiditySample {
        HumiditySample {
            humidity,
            temperature,
            pressure: None,
            updated_ms: 1,
            hum_filter: crate::sensor_filter::FieldFilter::default(),
            temp_filter: crate::sensor_filter::FieldFilter::default(),
        }
    }

    fn side(samples: &[(&str, Option<f32>, Option<f32>)]) -> HashMap<String, HumiditySample> {
        samples
            .iter()
            .map(|(topic, rh, t)| (topic.to_string(), sample(*rh, *t)))
            .collect()
    }

    #[test]
    fn muggy_cooler_outdoor_does_not_ventilate() {
        // Outdoor higher energy (humid) despite being cooler → hold down.
        let inp = HumidityInputs { indoor: air(55.0, 11.0, 55.0), outdoor: air(85.0, 13.0, 58.0) };
        let (lvl, cooling) = enthalpy_level(&inp, false);
        assert_eq!(lvl, VentLevel::Off);
        assert!(!cooling);
    }

    #[test]
    fn warm_but_drier_outdoor_assists_cooling() {
        // Outdoor lower energy (dry) → ventilate, cooling.
        let inp = HumidityInputs { indoor: air(60.0, 13.0, 60.0), outdoor: air(30.0, 9.0, 54.0) };
        let (lvl, cooling) = enthalpy_level(&inp, false);
        assert_eq!(lvl, VentLevel::Party);
        assert!(cooling);
    }

    // Only `h` matters to enthalpy_level; rh/ah are placeholders here.
    fn at_dh(indoor_h: f32, outdoor_h: f32) -> HumidityInputs {
        HumidityInputs { indoor: air(50.0, 10.0, indoor_h), outdoor: air(50.0, 10.0, outdoor_h) }
    }

    #[test]
    fn not_cooling_holds_reduced_in_neutral_band() {
        // Δh = -0.5 (in (H_COOL_ON, H_COOL_OFF]) and not previously cooling → hold.
        let (lvl, cooling) = enthalpy_level(&at_dh(60.0, 59.5), false);
        assert_eq!(lvl, VentLevel::Reduced);
        assert!(!cooling);
    }

    #[test]
    fn not_cooling_enters_cooling_below_on_threshold() {
        // Δh = -1.5 (< H_COOL_ON) → switch into cooling.
        let (lvl, cooling) = enthalpy_level(&at_dh(60.0, 58.5), false);
        assert_eq!(lvl, VentLevel::Normal);
        assert!(cooling);
    }

    #[test]
    fn cooling_holds_normal_in_neutral_band() {
        // Δh = -0.5 (above H_COOL_OFF would release, but it's below) and already
        // cooling → stay cooling.
        let (lvl, cooling) = enthalpy_level(&at_dh(60.0, 59.5), true);
        assert_eq!(lvl, VentLevel::Normal);
        assert!(cooling);
    }

    #[test]
    fn cooling_releases_above_off_threshold() {
        // Δh = +0.5 (> H_COOL_OFF) → release back to hold-down.
        let (lvl, cooling) = enthalpy_level(&at_dh(60.0, 60.5), true);
        assert_eq!(lvl, VentLevel::Reduced);
        assert!(!cooling);
    }

    #[test]
    fn live_event_does_not_flip_on_noise() {
        // The two real readings: indoor h≈63.2, outdoor 63.5 then 63.3, never
        // previously cooling. Both must stay Reduced/not-cooling (no muggy↔cooling
        // flip), since Δh (+0.3, +0.1) stays inside the neutral band.
        for outdoor in [63.5_f32, 63.3] {
            let (lvl, cooling) = enthalpy_level(&at_dh(63.2, outdoor), false);
            assert_eq!(lvl, VentLevel::Reduced, "outdoor h = {outdoor}");
            assert!(!cooling, "outdoor h = {outdoor}");
        }
    }

    #[test]
    fn deadband_holding_only_when_reduced_and_marginally_negative() {
        let reduced = VentLevel::Reduced as u8;
        let normal = VentLevel::Normal as u8;
        // Holding Reduced while Δh slipped to -0.5: the bare rule would cool → flag.
        assert!(deadband_holding(-0.5, reduced));
        // Δh clearly positive: genuinely reducing, not a withheld switch.
        assert!(!deadband_holding(0.5, reduced));
        // Already cooling (Normal): nothing is being withheld.
        assert!(!deadband_holding(-0.5, normal));
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
    fn air_at_temp_is_identity_at_sensor_temp() {
        // Re-expressing at the sensor's own temperature changes nothing.
        let a = air_at_temp(29.0, 36.6, 29.0, 96.1);
        assert!((a.rh - 36.6).abs() < 0.01, "rh = {}", a.rh);
    }

    #[test]
    fn air_at_cooler_temp_raises_rh_and_lowers_enthalpy() {
        // Same moisture, evaluated at a cooler temp: RH rises, enthalpy drops.
        let warm = air_at_temp(29.0, 36.6, 29.0, 96.1);
        let cool = air_at_temp(29.0, 36.6, 24.0, 96.1);
        assert!(cool.rh > warm.rh, "{} !> {}", cool.rh, warm.rh);
        assert!(cool.h < warm.h, "{} !< {}", cool.h, warm.h);
    }

    #[test]
    fn inlet_temp_correction_clears_phantom_muggy() {
        // Reproduces last night's 02:42 event. The garden DHT read ~32 °C / 33 %
        // while the real inlet (ID 80) was 25.1 °C; indoor 26.6 °C / 48 %.
        let indoor = {
            let ah = psychro::absolute_humidity(26.6, 48.0);
            let h = psychro::enthalpy(26.6, 48.0, 96.1);
            air(48.0, ah, h)
        };
        // Using the DHT's own (inflated) temperature -> outdoor looks energy-rich -> muggy.
        let outdoor_dht = air_at_temp(32.0, 33.0, 32.0, 96.1);
        let (_lvl, cooling_dht) = enthalpy_level(&HumidityInputs { indoor, outdoor: outdoor_dht }, false);
        assert!(!cooling_dht, "DHT temp should (wrongly) read as muggy");
        // Re-expressed at the true cool inlet temperature -> ventilating cools.
        let outdoor_inlet = air_at_temp(32.0, 33.0, 25.1, 96.1);
        let (_lvl, cooling_inlet) = enthalpy_level(&HumidityInputs { indoor, outdoor: outdoor_inlet }, false);
        assert!(cooling_inlet, "inlet temp should read as cooling, not muggy");
    }

    #[test]
    fn aggregate_takes_lowest_temp_and_highest_humidity() {
        // Two outdoor sensors: a cool real one and a hot garden-house DHT. The
        // aggregate must use the lowest temp and the highest humidity.
        let sensors = side(&[("real_outdoor", Some(30.0), Some(24.0)), ("garden_dht", Some(36.0), Some(29.0))]);
        // fans off so the inlet only acts as a last resort, not a temp candidate.
        let a = aggregate_side(&sensors, 1, 99.0, false, 96.1).unwrap();
        assert!((a.temp - 24.0).abs() < 0.01, "temp = {}", a.temp);
        assert!((a.rh - 36.0).abs() < 0.01, "rh = {}", a.rh);
    }

    #[test]
    fn aggregate_temp_only_sensor_feeds_temperature_not_moisture() {
        // A humidity-less outdoor thermometer (cool) plus the hot garden DHT for
        // humidity: temperature comes from the thermometer, humidity from the DHT.
        let sensors = side(&[("outdoor_thermo", None, Some(22.0)), ("garden_dht", Some(36.0), Some(29.0))]);
        let a = aggregate_side(&sensors, 1, 99.0, false, 96.1).unwrap();
        assert!((a.temp - 22.0).abs() < 0.01, "temp = {}", a.temp);
        assert!((a.rh - 36.0).abs() < 0.01, "rh = {}", a.rh);
    }

    #[test]
    fn aggregate_inlet_joins_temp_pool_only_when_fans_run() {
        // One hot DHT (29 C); the true inlet is 24 C.
        let sensors = side(&[("garden_dht", Some(36.0), Some(29.0))]);
        let off = aggregate_side(&sensors, 1, 24.0, false, 96.1).unwrap();
        assert!((off.temp - 29.0).abs() < 0.01, "fans off keeps sensor temp: {}", off.temp);
        let on = aggregate_side(&sensors, 1, 24.0, true, 96.1).unwrap();
        assert!((on.temp - 24.0).abs() < 0.01, "fans on adopt cool inlet: {}", on.temp);
    }

    #[test]
    fn aggregate_ignores_implausible_temperature() {
        // A dead sensor reporting -50 C must not drag the aggregate temperature down.
        let sensors = side(&[("good", Some(40.0), Some(26.0)), ("dead", Some(40.0), Some(-50.0))]);
        let a = aggregate_side(&sensors, 1, 99.0, false, 96.1).unwrap();
        assert!((a.temp - 26.0).abs() < 0.01, "temp = {}", a.temp);
    }

    #[test]
    fn aggregate_none_without_fresh_sensor() {
        let empty: HashMap<String, HumiditySample> = HashMap::new();
        assert!(aggregate_side(&empty, 1, 24.0, true, 96.1).is_none());
    }

    #[test]
    fn aggregate_none_when_only_implausible_fallback_temp() {
        // A humidity-only sensor and an implausible inlet (a boot/garbage temp)
        // must not fabricate an air state from that temperature.
        let sensors = side(&[("hum_only", Some(50.0), None)]);
        assert!(aggregate_side(&sensors, 1, 999.0, false, 96.1).is_none());
    }

    #[test]
    fn protection_hysteresis_holds_below_threshold_when_active() {
        // RH 62 (< RH_PROTECT 65) but already active → stays on (>= 60); drier outside.
        let inp = HumidityInputs { indoor: air(62.0, 12.0, 58.0), outdoor: air(40.0, 9.0, 54.0) };
        assert!(protection_level(&inp, true).is_some());
        assert!(protection_level(&inp, false).is_none()); // not yet active → don't trigger
    }
}
