//! Application configuration — mirrors the C++ AppConfig struct.

/// Maximum string field lengths (matching C++ defines)
pub const MAX_SSID_LEN: usize = 33;
pub const MAX_PASSWORD_LEN: usize = 65;
pub const MAX_HOST_LEN: usize = 64;
pub const MAX_USERNAME_LEN: usize = 64;
pub const MAX_TOPIC_LEN: usize = 128;

/// Application configuration
#[derive(Debug, Clone)]
pub struct AppConfig {
    // WiFi
    pub wifi_ssid: String,
    pub wifi_password: String,

    // MQTT
    pub mqtt_enabled: bool,
    pub mqtt_server: String,
    pub mqtt_port: u16,
    pub mqtt_topic: String,
    pub mqtt_username: String,
    pub mqtt_password: String,
    pub mqtt_auth_enabled: bool,

    // Web UI auth
    pub web_username: String,
    pub web_password: String,

    // Pin configuration
    pub ot_in_pin: u8,
    pub ot_out_pin: u8,
    pub sda_pin: u8,
    pub scl_pin: u8,
    pub enc_clk_pin: u8,
    pub enc_dt_pin: u8,
    pub enc_sw_pin: u8,

    // Ventilation state (persisted)
    pub ventilation_level: u8,
    pub bypass_open: bool,

    // System
    pub configured: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            wifi_ssid: String::new(),
            wifi_password: String::new(),
            mqtt_enabled: false,
            mqtt_server: String::new(),
            mqtt_port: 1883,
            mqtt_topic: "wolf-cwl".to_string(),
            mqtt_username: String::new(),
            mqtt_password: String::new(),
            mqtt_auth_enabled: false,
            web_username: "admin".to_string(),
            web_password: "admin".to_string(),
            ot_in_pin: 36,
            ot_out_pin: 4,
            sda_pin: 13,
            scl_pin: 16,
            enc_clk_pin: 15,
            enc_dt_pin: 14,
            enc_sw_pin: 5,
            ventilation_level: 2, // Normal
            bypass_open: false,
            configured: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.mqtt_port, 1883);
        assert_eq!(config.mqtt_topic, "wolf-cwl");
        assert_eq!(config.web_username, "admin");
        assert_eq!(config.web_password, "admin");
        assert_eq!(config.ventilation_level, 2);
        assert!(!config.bypass_open);
        assert!(!config.configured);
    }
}
