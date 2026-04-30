//! NVS-based configuration persistence.
//!
//! Uses the same namespace ("wolfcwl") and key names as the C++ firmware
//! for migration compatibility.

use esp_idf_svc::nvs::{EspDefaultNvsPartition, EspNvs, NvsDefault};
use log::info;

use crate::config::AppConfig;
use crate::scheduler::{ScheduleEntry, BypassSchedule};

const NVS_NAMESPACE: &str = "wolfcwl";

pub struct ConfigManager {
    nvs: EspNvs<NvsDefault>,
}

impl ConfigManager {
    pub fn new(partition: EspDefaultNvsPartition) -> Result<Self, esp_idf_svc::sys::EspError> {
        let nvs = EspNvs::new(partition, NVS_NAMESPACE, true)?;
        Ok(Self { nvs })
    }

    /// Load configuration from NVS. Returns default config if not configured.
    pub fn load(&self) -> AppConfig {
        let mut config = AppConfig::default();

        config.configured = self.get_bool("configured", false);
        if !config.configured {
            info!("Config: Not configured, using defaults");
            return config;
        }

        info!("Config: Loading from NVS...");

        // WiFi
        config.wifi_ssid = self.get_string("wifi_ssid", "");
        config.wifi_password = self.get_string("wifi_pass", "");

        // MQTT
        config.mqtt_enabled = self.get_bool("mqtt_enabled", false);
        config.mqtt_server = self.get_string("mqtt_server", "");
        config.mqtt_port = self.get_i32("mqtt_port", 1883) as u16;
        config.mqtt_topic = self.get_string("mqtt_topic", "wolf-cwl");
        config.mqtt_username = self.get_string("mqtt_user", "");
        config.mqtt_password = self.get_string("mqtt_pass", "");
        config.mqtt_auth_enabled = self.get_bool("mqtt_auth", false);

        // Web UI auth
        config.web_username = self.get_string("web_user", "admin");
        config.web_password = self.get_string("web_pass", "admin");

        // Pin configuration
        config.ot_in_pin = self.get_i32("ot_in_pin", config.ot_in_pin as i32) as u8;
        config.ot_out_pin = self.get_i32("ot_out_pin", config.ot_out_pin as i32) as u8;
        config.sda_pin = self.get_i32("sda_pin", config.sda_pin as i32) as u8;
        config.scl_pin = self.get_i32("scl_pin", config.scl_pin as i32) as u8;
        config.enc_clk_pin = self.get_i32("enc_clk_pin", config.enc_clk_pin as i32) as u8;
        config.enc_dt_pin = self.get_i32("enc_dt_pin", config.enc_dt_pin as i32) as u8;
        config.enc_sw_pin = self.get_i32("enc_sw_pin", config.enc_sw_pin as i32) as u8;

        // Ventilation state
        config.ventilation_level = self.get_i32("vent_level", 2) as u8;
        config.bypass_open = self.get_bool("bypass_open", false);

        info!("Config: Loaded from NVS");
        config
    }

    /// Save full configuration to NVS.
    pub fn save(&mut self, config: &AppConfig) -> Result<(), esp_idf_svc::sys::EspError> {
        // WiFi
        self.set_string("wifi_ssid", &config.wifi_ssid)?;
        self.set_string("wifi_pass", &config.wifi_password)?;

        // MQTT
        self.set_bool("mqtt_enabled", config.mqtt_enabled)?;
        self.set_string("mqtt_server", &config.mqtt_server)?;
        self.set_i32("mqtt_port", config.mqtt_port as i32)?;
        self.set_string("mqtt_topic", &config.mqtt_topic)?;
        self.set_string("mqtt_user", &config.mqtt_username)?;
        self.set_string("mqtt_pass", &config.mqtt_password)?;
        self.set_bool("mqtt_auth", config.mqtt_auth_enabled)?;

        // Web UI auth
        self.set_string("web_user", &config.web_username)?;
        self.set_string("web_pass", &config.web_password)?;

        // Pin configuration
        self.set_i32("ot_in_pin", config.ot_in_pin as i32)?;
        self.set_i32("ot_out_pin", config.ot_out_pin as i32)?;
        self.set_i32("sda_pin", config.sda_pin as i32)?;
        self.set_i32("scl_pin", config.scl_pin as i32)?;
        self.set_i32("enc_clk_pin", config.enc_clk_pin as i32)?;
        self.set_i32("enc_dt_pin", config.enc_dt_pin as i32)?;
        self.set_i32("enc_sw_pin", config.enc_sw_pin as i32)?;

        // Ventilation state
        self.set_i32("vent_level", config.ventilation_level as i32)?;
        self.set_bool("bypass_open", config.bypass_open)?;

        // Mark as configured
        self.set_bool("configured", config.configured)?;

        info!("Config: Saved to NVS");
        Ok(())
    }

    /// Save only the bypass state (fast, single-key write).
    #[allow(dead_code)]
    pub fn save_bypass_state(&mut self, bypass_open: bool) -> Result<(), esp_idf_svc::sys::EspError> {
        self.set_bool("bypass_open", bypass_open)
    }

    /// Clear all NVS keys (factory reset).
    pub fn reset(&mut self) -> Result<(), esp_idf_svc::sys::EspError> {
        // EspNvs doesn't have a clear-all, so we reset key by key
        // by saving a default config with configured=false
        let default_config = AppConfig::default();
        self.save(&default_config)?;
        self.set_bool("configured", false)?;
        info!("Config: Factory reset complete");
        Ok(())
    }

    /// Load ventilation schedules from NVS.
    pub fn load_schedules(&self) -> Vec<ScheduleEntry> {
        let json = self.get_string("schedules", "[]");
        serde_json::from_str(&json).unwrap_or_default()
    }

    /// Save ventilation schedules to NVS.
    #[allow(dead_code)]
    pub fn save_schedules(&mut self, schedules: &[ScheduleEntry]) -> Result<(), esp_idf_svc::sys::EspError> {
        let json = serde_json::to_string(schedules).unwrap_or_else(|_| "[]".into());
        self.set_string("schedules", &json)
    }

    /// Load bypass schedule from NVS.
    pub fn load_bypass_schedule(&self) -> BypassSchedule {
        let json = self.get_string("bypass_sched", "{}");
        serde_json::from_str(&json).unwrap_or_default()
    }

    /// Save bypass schedule to NVS.
    #[allow(dead_code)]
    pub fn save_bypass_schedule(&mut self, schedule: &BypassSchedule) -> Result<(), esp_idf_svc::sys::EspError> {
        let json = serde_json::to_string(schedule).unwrap_or_else(|_| "{}".into());
        self.set_string("bypass_sched", &json)
    }

    /// Load timed-off end epoch from NVS (0 = inactive).
    pub fn load_timed_off_end(&self) -> i64 {
        // Store as two i32 keys (NVS doesn't support i64 directly)
        let lo = self.get_i32("toff_end_lo", 0) as u32;
        let hi = self.get_i32("toff_end_hi", 0) as u32;
        ((hi as i64) << 32) | (lo as i64)
    }

    /// Save timed-off end epoch to NVS.
    pub fn save_timed_off_end(&mut self, end_epoch: i64) -> Result<(), esp_idf_svc::sys::EspError> {
        self.set_i32("toff_end_lo", end_epoch as i32)?;
        self.set_i32("toff_end_hi", (end_epoch >> 32) as i32)
    }

    /// Load schedule override state from NVS.
    pub fn load_override_state(&self) -> bool {
        self.get_bool("sched_ovr", false)
    }

    /// Save schedule override state to NVS.
    pub fn save_override_state(&mut self, override_active: bool) -> Result<(), esp_idf_svc::sys::EspError> {
        self.set_bool("sched_ovr", override_active)
    }

    // --- NVS helpers ---

    fn get_string(&self, key: &str, default: &str) -> String {
        let mut buf = [0u8; 256];
        match self.nvs.get_str(key, &mut buf) {
            Ok(Some(s)) => s.trim_end_matches('\0').to_string(),
            _ => default.to_string(),
        }
    }

    fn get_bool(&self, key: &str, default: bool) -> bool {
        match self.nvs.get_u8(key) {
            Ok(Some(v)) => v != 0,
            _ => default,
        }
    }

    fn get_i32(&self, key: &str, default: i32) -> i32 {
        match self.nvs.get_i32(key) {
            Ok(Some(v)) => v,
            _ => default,
        }
    }

    fn set_string(&mut self, key: &str, value: &str) -> Result<(), esp_idf_svc::sys::EspError> {
        self.nvs.set_str(key, value)
    }

    fn set_bool(&mut self, key: &str, value: bool) -> Result<(), esp_idf_svc::sys::EspError> {
        self.nvs.set_u8(key, if value { 1 } else { 0 })
    }

    fn set_i32(&mut self, key: &str, value: i32) -> Result<(), esp_idf_svc::sys::EspError> {
        self.nvs.set_i32(key, value)
    }
}
