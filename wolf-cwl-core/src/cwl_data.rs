//! Wolf CWL data model — mirrors the C++ CwlData struct.

/// Maximum TSP register index
pub const TSP_MAX_INDEX: usize = 69;

/// Ventilation levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum VentLevel {
    Off = 0,
    Reduced = 1,
    Normal = 2,
    Party = 3,
}

impl VentLevel {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Off,
            1 => Self::Reduced,
            2 => Self::Normal,
            3 => Self::Party,
            _ => Self::Normal,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Reduced => "Reduced",
            Self::Normal => "Normal",
            Self::Party => "Party",
        }
    }
}

/// Central data model for Wolf CWL ventilation unit
#[derive(Debug)]
pub struct CwlData {
    // Temperatures (f8.8 format, °C)
    pub supply_inlet_temp: f32,     // ID 80
    pub exhaust_inlet_temp: f32,    // ID 82
    pub supply_outlet_temp: f32,    // ID 81 (if supported)
    pub exhaust_outlet_temp: f32,   // ID 83 (if supported)

    // Ventilation
    pub ventilation_level: u8,       // ID 71: 0-3
    pub relative_ventilation: u8,    // ID 77: 0-100%

    // Status flags from ID 70 response
    pub fault: bool,
    pub ventilation_active: bool,    // bypass active
    pub cooling_active: bool,
    pub dhw_active: bool,
    pub filter_dirty: bool,
    pub diag_event: bool,

    // Fault info from ID 72
    pub fault_flags: u8,
    pub oem_fault_code: u8,

    // Config from ID 74
    pub slave_flags: u8,
    pub slave_member_id: u8,

    // Fan speeds (if supported)
    pub exhaust_fan_speed: u16,      // ID 84
    pub supply_fan_speed: u16,       // ID 85

    // TSP register cache
    pub tsp_values: [u8; TSP_MAX_INDEX],
    pub tsp_valid: [bool; TSP_MAX_INDEX],

    // Product versions
    pub master_type: u8,
    pub master_version: u8,
    pub slave_type: u8,
    pub slave_version: u8,

    // Probed data ID support
    pub supports_id78: bool,  // Relative humidity
    pub supports_id79: bool,  // CO2 level
    pub supports_id81: bool,  // Supply outlet temp
    pub supports_id83: bool,  // Exhaust outlet temp
    pub supports_id84: bool,  // Exhaust fan speed
    pub supports_id85: bool,  // Supply fan speed
    pub supports_id87: bool,  // Nominal ventilation
    pub supports_id88: bool,  // TSP count

    // Derived values from TSP registers
    pub current_volume: u16,         // TSP 52,53: m³/h
    pub bypass_status: u8,           // TSP 54
    pub temp_atmospheric: i8,        // TSP 55: °C
    pub temp_indoor: i8,             // TSP 56: °C
    pub input_duct_pressure: u16,    // TSP 64,65: Pa
    pub output_duct_pressure: u16,   // TSP 66,67: Pa
    pub frost_status: u8,            // TSP 68

    // Connection state
    pub connected: bool,
    pub last_response_ms: u32,
}

impl Default for CwlData {
    fn default() -> Self {
        Self {
            supply_inlet_temp: 0.0,
            exhaust_inlet_temp: 0.0,
            supply_outlet_temp: 0.0,
            exhaust_outlet_temp: 0.0,
            ventilation_level: VentLevel::Normal as u8,
            relative_ventilation: 0,
            fault: false,
            ventilation_active: false,
            cooling_active: false,
            dhw_active: false,
            filter_dirty: false,
            diag_event: false,
            fault_flags: 0,
            oem_fault_code: 0,
            slave_flags: 0,
            slave_member_id: 0,
            exhaust_fan_speed: 0,
            supply_fan_speed: 0,
            tsp_values: [0; TSP_MAX_INDEX],
            tsp_valid: [false; TSP_MAX_INDEX],
            master_type: 0,
            master_version: 0,
            slave_type: 0,
            slave_version: 0,
            supports_id78: false,
            supports_id79: false,
            supports_id81: false,
            supports_id83: false,
            supports_id84: false,
            supports_id85: false,
            supports_id87: false,
            supports_id88: false,
            current_volume: 0,
            bypass_status: 0,
            temp_atmospheric: 0,
            temp_indoor: 0,
            input_duct_pressure: 0,
            output_duct_pressure: 0,
            frost_status: 0,
            connected: false,
            last_response_ms: 0,
        }
    }
}

impl CwlData {
    pub fn new() -> Self {
        Self::default()
    }

    /// Update derived values from TSP register cache
    pub fn update_derived_values(&mut self) {
        // Current volume (TSP 52, 53) — 16-bit little-endian
        if self.tsp_valid[52] && self.tsp_valid[53] {
            self.current_volume =
                self.tsp_values[52] as u16 | ((self.tsp_values[53] as u16) << 8);
        }

        // Bypass status (TSP 54)
        if self.tsp_valid[54] {
            self.bypass_status = self.tsp_values[54];
        }

        // Atmospheric temperature (TSP 55): value - 100 = °C
        if self.tsp_valid[55] {
            self.temp_atmospheric = self.tsp_values[55] as i8 - 100;
        }

        // Indoor temperature (TSP 56): value - 100 = °C
        if self.tsp_valid[56] {
            self.temp_indoor = self.tsp_values[56] as i8 - 100;
        }

        // Input duct pressure (TSP 64, 65)
        if self.tsp_valid[64] && self.tsp_valid[65] {
            self.input_duct_pressure =
                self.tsp_values[64] as u16 | ((self.tsp_values[65] as u16) << 8);
        }

        // Output duct pressure (TSP 66, 67)
        if self.tsp_valid[66] && self.tsp_valid[67] {
            self.output_duct_pressure =
                self.tsp_values[66] as u16 | ((self.tsp_values[67] as u16) << 8);
        }

        // Frost status (TSP 68)
        if self.tsp_valid[68] {
            self.frost_status = self.tsp_values[68];
        }
    }
}

/// Decode f8.8 fixed-point temperature from OpenTherm 16-bit value
pub fn decode_f88(value: u16) -> f32 {
    let hi = (value >> 8) as i8;
    let lo = (value & 0xFF) as u8;
    hi as f32 + lo as f32 / 256.0
}

/// Get the name for a ventilation level
pub fn ventilation_level_name(level: u8) -> &'static str {
    VentLevel::from_u8(level).name()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_f88() {
        // 18.5°C = 0x1280 (18 * 256 + 128)
        assert!((decode_f88(0x1280) - 18.5).abs() < 0.01);

        // 0°C
        assert!((decode_f88(0x0000) - 0.0).abs() < 0.01);

        // -1°C = 0xFF00
        assert!((decode_f88(0xFF00) - (-1.0)).abs() < 0.01);

        // 21.25°C = 0x1540
        assert!((decode_f88(0x1540) - 21.25).abs() < 0.01);
    }

    #[test]
    fn test_vent_level_from_u8() {
        assert_eq!(VentLevel::from_u8(0), VentLevel::Off);
        assert_eq!(VentLevel::from_u8(1), VentLevel::Reduced);
        assert_eq!(VentLevel::from_u8(2), VentLevel::Normal);
        assert_eq!(VentLevel::from_u8(3), VentLevel::Party);
        assert_eq!(VentLevel::from_u8(99), VentLevel::Normal); // fallback
    }

    #[test]
    fn test_vent_level_name() {
        assert_eq!(ventilation_level_name(0), "Off");
        assert_eq!(ventilation_level_name(1), "Reduced");
        assert_eq!(ventilation_level_name(2), "Normal");
        assert_eq!(ventilation_level_name(3), "Party");
    }

    #[test]
    fn test_update_derived_values() {
        let mut data = CwlData::new();

        // Set TSP 52=130, 53=0 → volume = 130 m³/h
        data.tsp_values[52] = 130;
        data.tsp_values[53] = 0;
        data.tsp_valid[52] = true;
        data.tsp_valid[53] = true;

        // TSP 55 = 114 → 14°C
        data.tsp_values[55] = 114;
        data.tsp_valid[55] = true;

        data.update_derived_values();

        assert_eq!(data.current_volume, 130);
        assert_eq!(data.temp_atmospheric, 14);
    }
}
