//! Psychrometrics — moisture and energy of moist air from temperature, relative
//! humidity and ambient pressure.
//!
//! Two metrics drive the humidity-aware ventilation logic:
//! - **Absolute humidity** (g/m³): how much water the air actually holds. Drives
//!   moisture protection (ventilating only helps if outside air holds less water).
//!   Independent of pressure.
//! - **Specific enthalpy** (kJ/kg dry air): the total energy of the air. Humid air
//!   carries more energy than dry air at the same temperature, so this — not bare
//!   temperature — is the right basis for "does ventilating cool?". Depends on
//!   ambient pressure (matters at altitude).

/// Standard sea-level pressure in kPa, used when no sensor reports pressure.
pub const STANDARD_PRESSURE_KPA: f32 = 101.325;

/// Saturation vapour pressure over water (kPa), Magnus form.
pub fn saturation_kpa(t: f32) -> f32 {
    0.61094 * (17.625 * t / (t + 243.04)).exp()
}

/// Actual vapour pressure (kPa) from temperature and relative humidity (%).
pub fn vapor_kpa(t: f32, rh: f32) -> f32 {
    (rh / 100.0) * saturation_kpa(t)
}

/// Absolute humidity in g/m³ from temperature (°C) and relative humidity (%).
pub fn absolute_humidity(t: f32, rh: f32) -> f32 {
    let p_v_pa = vapor_kpa(t, rh) * 1000.0;
    2.16679 * p_v_pa / (t + 273.15)
}

/// Specific enthalpy of moist air in kJ/kg dry air, given temperature (°C),
/// relative humidity (%) and ambient pressure (kPa).
pub fn enthalpy(t: f32, rh: f32, p_atm_kpa: f32) -> f32 {
    let p_v = vapor_kpa(t, rh);
    let p_atm = if p_atm_kpa > 1.0 { p_atm_kpa } else { STANDARD_PRESSURE_KPA };
    // Humidity ratio (kg water / kg dry air). Guard the denominator.
    let denom = (p_atm - p_v).max(0.1);
    let w = 0.622 * p_v / denom;
    1.006 * t + w * (2501.0 + 1.86 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absolute_humidity_known_point() {
        // 20 °C / 50 % ≈ 8.6 g/m³
        assert!((absolute_humidity(20.0, 50.0) - 8.6).abs() < 0.2);
    }

    #[test]
    fn absolute_humidity_rises_with_humidity() {
        assert!(absolute_humidity(27.0, 80.0) > absolute_humidity(27.0, 30.0));
    }

    #[test]
    fn enthalpy_indoor_example() {
        // 27 °C / 55 % at standard pressure ≈ 58 kJ/kg
        let h = enthalpy(27.0, 55.0, STANDARD_PRESSURE_KPA);
        assert!((h - 58.0).abs() < 2.0, "h = {}", h);
    }

    #[test]
    fn enthalpy_hot_dry_outdoor_example() {
        // Garden sensor 38.7 °C / 30.5 % — hot but fairly dry.
        let h = enthalpy(38.7, 30.5, STANDARD_PRESSURE_KPA);
        // Sanity band for that state (~78–86 kJ/kg).
        assert!(h > 75.0 && h < 90.0, "h = {}", h);
    }

    #[test]
    fn humid_air_carries_more_energy_than_dry_at_same_temp() {
        // Core physics: at equal temperature, humid air has higher enthalpy.
        let humid = enthalpy(28.0, 80.0, STANDARD_PRESSURE_KPA);
        let dry = enthalpy(28.0, 30.0, STANDARD_PRESSURE_KPA);
        assert!(humid > dry);
    }

    #[test]
    fn lower_pressure_raises_humidity_ratio() {
        // At altitude (lower p_atm) the same T/RH yields higher W → higher enthalpy.
        let sea = enthalpy(27.0, 55.0, 101.3);
        let alt = enthalpy(27.0, 55.0, 96.0);
        assert!(alt > sea);
    }
}
