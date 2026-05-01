//! MQTT client — publish sensor data, subscribe to commands.

use std::sync::{Arc, Mutex};

use esp_idf_svc::mqtt::client::{EspMqttClient, EspMqttEvent, EventPayload, MqttClientConfiguration, QoS};
use log::{info, warn};

use crate::app_state::AppStateInner;
use crate::cwl_data::ventilation_level_name;

type AppState = Arc<Mutex<AppStateInner>>;

const SENSOR_INTERVAL_MS: u32 = 11_000;
const HEALTH_INTERVAL_MS: u32 = 60_000;

pub struct MqttManager {
    client: EspMqttClient<'static>,
    base_topic: String,
    state: AppState,
    last_publish_ms: u32,
    last_health_ms: u32,
    connected: bool,
}

impl MqttManager {
    pub fn new(state: AppState) -> Option<Self> {
        let (mqtt_enabled, mqtt_server, mqtt_port, mqtt_topic, _mqtt_auth, _mqtt_user, _mqtt_pass) = {
            let st = state.lock().unwrap();
            (
                st.config.mqtt_enabled,
                st.config.mqtt_server.clone(),
                st.config.mqtt_port,
                st.config.mqtt_topic.clone(),
                st.config.mqtt_auth_enabled,
                st.config.mqtt_username.clone(),
                st.config.mqtt_password.clone(),
            )
        };

        if !mqtt_enabled || mqtt_server.is_empty() {
            info!("MQTT: Disabled or no server configured");
            return None;
        }

        let url = format!("mqtt://{}:{}", mqtt_server, mqtt_port);
        let base_topic = mqtt_topic;

        let conf = MqttClientConfiguration {
            ..Default::default()
        };

        info!("MQTT: Connecting to {}...", url);

        let s = state.clone();
        let base = base_topic.clone();
        let client = match EspMqttClient::new_cb(
            &url,
            &conf,
            move |event| {
                handle_event(event, &s, &base);
            },
        ) {
            Ok(c) => c,
            Err(e) => {
                warn!("MQTT: Connection failed: {:?}", e);
                return None;
            }
        };

        info!("MQTT: Client created");

        Some(Self {
            client,
            base_topic,
            state,
            last_publish_ms: 0,
            last_health_ms: 0,
            connected: true,
        })
    }

    pub fn setup_subscriptions(&mut self) {
        let topics = [
            format!("{}/set/level", self.base_topic),
            format!("{}/set/bypass", self.base_topic),
            format!("{}/set/filter_reset", self.base_topic),
            format!("{}/set/off_timer", self.base_topic),
        ];
        for topic in &topics {
            if let Err(e) = self.client.subscribe(topic, QoS::AtMostOnce) {
                warn!("MQTT: Subscribe to {} failed: {:?}", topic, e);
            }
        }
        info!("MQTT: Subscribed to command topics");

        // Publish bridge info
        self.pub_retained("bridge/state", "online");
        self.pub_retained("bridge/version", env!("CARGO_PKG_VERSION"));
        {
            let st = self.state.lock().unwrap();
            if let Some(ref ip) = st.ip_address {
                let ip_clone = ip.clone();
                drop(st);
                self.pub_retained("bridge/ip", &ip_clone);
            }
        }
    }

    pub fn update(&mut self, now_ms: u32) {
        if !self.connected {
            return;
        }

        if now_ms.wrapping_sub(self.last_publish_ms) >= SENSOR_INTERVAL_MS {
            self.last_publish_ms = now_ms;
            self.publish_sensor_data();
        }

        if now_ms.wrapping_sub(self.last_health_ms) >= HEALTH_INTERVAL_MS {
            self.last_health_ms = now_ms;
            self.publish_health_data();
        }
    }

    fn publish_sensor_data(&mut self) {
        let st = self.state.lock().unwrap();
        let d = &st.cwl_data;

        // Collect all topic/payload pairs while holding the lock
        let mut msgs: Vec<(String, String)> = Vec::with_capacity(25);

        msgs.push(("ventilation/level".into(), d.ventilation_level.to_string()));
        msgs.push(("ventilation/level_name".into(), ventilation_level_name(d.ventilation_level).into()));
        msgs.push(("ventilation/relative".into(), d.relative_ventilation.to_string()));
        msgs.push(("temperature/supply_inlet".into(), format!("{:.1}", d.supply_inlet_temp)));
        msgs.push(("temperature/exhaust_inlet".into(), format!("{:.1}", d.exhaust_inlet_temp)));

        if d.supports_id81 { msgs.push(("temperature/supply_outlet".into(), format!("{:.1}", d.supply_outlet_temp))); }
        if d.supports_id83 { msgs.push(("temperature/exhaust_outlet".into(), format!("{:.1}", d.exhaust_outlet_temp))); }

        msgs.push(("status/fault".into(), if d.fault { "1" } else { "0" }.into()));
        msgs.push(("status/filter".into(), if d.filter_dirty { "1" } else { "0" }.into()));
        msgs.push(("status/bypass".into(), if d.ventilation_active { "1" } else { "0" }.into()));

        if d.supports_id84 { msgs.push(("fan/exhaust_speed".into(), d.exhaust_fan_speed.to_string())); }
        if d.supports_id85 { msgs.push(("fan/supply_speed".into(), d.supply_fan_speed.to_string())); }

        if d.tsp_valid[52] { msgs.push(("airflow/current_volume".into(), d.current_volume.to_string())); }
        if d.tsp_valid[55] { msgs.push(("temperature/atmospheric".into(), d.temp_atmospheric.to_string())); }
        if d.tsp_valid[56] { msgs.push(("temperature/indoor".into(), d.temp_indoor.to_string())); }
        if d.tsp_valid[64] { msgs.push(("pressure/input_duct".into(), d.input_duct_pressure.to_string())); }
        if d.tsp_valid[66] { msgs.push(("pressure/output_duct".into(), d.output_duct_pressure.to_string())); }
        if d.tsp_valid[68] { msgs.push(("status/frost".into(), d.frost_status.to_string())); }

        msgs.push(("schedule/active".into(), if st.schedule_active { "1" } else { "0" }.into()));
        msgs.push(("schedule/override".into(), if st.schedule_override { "1" } else { "0" }.into()));
        msgs.push(("bypass/mode".into(), if st.requested_bypass_open { "summer" } else { "winter" }.into()));

        msgs.push(("off_timer/active".into(), if st.timed_off_active { "1" } else { "0" }.into()));
        msgs.push(("off_timer/remaining".into(), st.timed_off_remaining_min.to_string()));
        msgs.push(("bypass/schedule_active".into(), "0".into()));
        msgs.push(("bypass/override".into(), "0".into()));

        if d.tsp_valid[54] { msgs.push(("status/bypass_position".into(), d.bypass_status.to_string())); }

        drop(st); // Release lock before publishing

        for (sub_topic, payload) in &msgs {
            self.pub_retained(sub_topic, payload);
        }
    }

    fn publish_health_data(&mut self) {
        let uptime = unsafe { esp_idf_svc::sys::esp_timer_get_time() / 1_000_000 };
        let free_heap = unsafe { esp_idf_svc::sys::esp_get_free_heap_size() };

        self.pub_retained("health/uptime", &uptime.to_string());
        self.pub_retained("health/free_heap", &free_heap.to_string());
        self.pub_retained("health/reboot_reason", crate::watchdog::reboot_reason());
        self.pub_retained("health/crash_count", "0");

        let ot_age = {
            let st = self.state.lock().unwrap();
            let now_ms = unsafe { (esp_idf_svc::sys::esp_timer_get_time() / 1000) as u32 };
            if st.cwl_data.last_response_ms > 0 {
                now_ms.wrapping_sub(st.cwl_data.last_response_ms)
            } else {
                0
            }
        };
        self.pub_retained("health/ot_response_age", &ot_age.to_string());
    }

    fn pub_retained(&mut self, sub_topic: &str, payload: &str) {
        let topic = format!("{}/{}", self.base_topic, sub_topic);
        let _ = self.client.publish(&topic, QoS::AtMostOnce, true, payload.as_bytes());
    }
}

fn handle_event(event: EspMqttEvent<'_>, state: &AppState, base_topic: &str) {
    match event.payload() {
        EventPayload::Connected(_) => {
            info!("MQTT: Connected");
            state.lock().unwrap().mqtt_connected = true;
        }
        EventPayload::Disconnected => {
            warn!("MQTT: Disconnected");
            state.lock().unwrap().mqtt_connected = false;
        }
        EventPayload::Received { topic, data, .. } => {
            if let Some(topic) = topic {
                let msg = std::str::from_utf8(data).unwrap_or("");
                handle_command(topic, msg, state, base_topic);
            }
        }
        _ => {}
    }
}

fn handle_command(topic: &str, message: &str, state: &AppState, _base_topic: &str) {
    let mut st = state.lock().unwrap();

    if topic.ends_with("/set/level") {
        if let Ok(level) = message.trim().parse::<u8>() {
            if level <= 3 {
                st.requested_vent_level = level;
                st.config.ventilation_level = level;
                st.display_wake_requested = true;
                info!("MQTT: Level set to {} ({})", level, ventilation_level_name(level));
            }
        }
    } else if topic.ends_with("/set/bypass") {
        let open = matches!(message.trim(), "1" | "true" | "on");
        st.requested_bypass_open = open;
        st.config.bypass_open = open;
        st.display_wake_requested = true;
        info!("MQTT: Bypass {}", if open { "open" } else { "closed" });
    } else if topic.ends_with("/set/filter_reset") {
        if matches!(message.trim(), "1" | "true") {
            st.requested_filter_reset = true;
            info!("MQTT: Filter reset triggered");
        }
    } else if topic.ends_with("/set/off_timer") {
        if let Ok(hours) = message.trim().parse::<u8>() {
            if hours >= 1 && hours <= 99 {
                st.timed_off_request = Some(hours);
                st.display_wake_requested = true;
                info!("MQTT: Timed off requested for {}h", hours);
            }
        }
    }
}
