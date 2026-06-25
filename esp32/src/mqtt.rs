//! MQTT client — publish sensor data, subscribe to commands.

use std::sync::{Arc, Mutex};

use esp_idf_svc::mqtt::client::{EspMqttClient, EspMqttEvent, EventPayload, MqttClientConfiguration, QoS};
use log::{info, warn};

use crate::app_state::{AppStateInner, HumiditySample};
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
    /// Tracks the broker connection's rising edge so we can (re)subscribe every
    /// time it comes up. The initial `setup_subscriptions()` runs before the
    /// first connect and the client does not auto-resubscribe, so without this
    /// the device intermittently misses the retained `persist/*` snapshots —
    /// which is how restored history was lost after a flash.
    last_connected: bool,
    /// Humidity-sensor topics we've already subscribed to (so config edits add
    /// new subscriptions on the next tick without a reboot).
    subscribed_sensors: Vec<String>,
}

/// Sensor barometers report hPa (e.g. 960.9); enthalpy wants kPa. Values that
/// already look like kPa (< 200) are passed through.
fn pressure_to_kpa(p: f32) -> f32 {
    if p > 200.0 { p / 10.0 } else { p }
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

        // Buffers sized to hold the retained history snapshot (~12 KB of
        // downsampled timestamped points) in a single frame, so neither the
        // publish nor the recovery read fragments.
        let conf = MqttClientConfiguration {
            buffer_size: 16384,
            out_buffer_size: 16384,
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
            last_connected: false,
            subscribed_sensors: Vec::new(),
        })
    }

    /// Subscribe to any configured humidity-sensor topics not yet subscribed.
    /// Idempotent: called at setup and each tick so config edits apply without
    /// a reboot.
    fn sync_sensor_subscriptions(&mut self) {
        let topics: Vec<String> = {
            let st = self.state.lock().unwrap();
            let mut v = st.config.humidity_inside_topics.clone();
            if !st.config.humidity_outside_topic.is_empty() {
                v.push(st.config.humidity_outside_topic.clone());
            }
            v
        };
        for t in topics {
            if t.is_empty() || self.subscribed_sensors.contains(&t) {
                continue;
            }
            if let Err(e) = self.client.subscribe(&t, QoS::AtMostOnce) {
                warn!("MQTT: Subscribe to sensor {} failed: {:?}", t, e);
            } else {
                info!("MQTT: Subscribed to humidity sensor {}", t);
                self.subscribed_sensors.push(t);
            }
        }
    }

    pub fn setup_subscriptions(&mut self) {
        let topics = [
            format!("{}/set/level", self.base_topic),
            format!("{}/set/bypass", self.base_topic),
            format!("{}/set/filter_reset", self.base_topic),
            format!("{}/set/off_timer", self.base_topic),
            // Retained persistence topics — subscribing pulls back any retained
            // snapshot so the device can recover history/markers into RAM.
            format!("{}/persist/temp_history", self.base_topic),
            format!("{}/persist/extreme_heat", self.base_topic),
        ];
        for topic in &topics {
            if let Err(e) = self.client.subscribe(topic, QoS::AtMostOnce) {
                warn!("MQTT: Subscribe to {} failed: {:?}", topic, e);
            }
        }
        info!("MQTT: Subscribed to command topics");
        self.sync_sensor_subscriptions();

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

        // (Re)subscribe on every broker connection rising edge. The client does
        // not auto-resubscribe after a reconnect, and the boot-time subscribe can
        // run before the link is up, so re-establishing here is what makes the
        // retained persist/* snapshots (history + extreme-heat) reliably arrive.
        let now_connected = self.state.lock().unwrap().mqtt_connected;
        if now_connected && !self.last_connected {
            self.setup_subscriptions();
            self.subscribed_sensors.clear();
        }
        self.last_connected = now_connected;

        // Pick up newly-configured sensor topics (cheap; no-op when unchanged).
        self.sync_sensor_subscriptions();

        if now_ms.wrapping_sub(self.last_publish_ms) >= SENSOR_INTERVAL_MS {
            self.last_publish_ms = now_ms;
            self.publish_sensor_data();
        }

        if now_ms.wrapping_sub(self.last_health_ms) >= HEALTH_INTERVAL_MS {
            self.last_health_ms = now_ms;
            self.publish_health_data();
        }

        // Retained RAM-only state persistence — published only when a producer
        // raised the flag (history bucket rollover / extreme-heat change), so we
        // never wipe the broker's retained copy with an empty/initial snapshot.
        let (do_history, do_extreme_heat) = {
            let mut st = self.state.lock().unwrap();
            let h = std::mem::take(&mut st.mqtt_publish_history);
            let e = std::mem::take(&mut st.mqtt_publish_extreme_heat);
            (h, e)
        };
        if do_history { self.publish_history_snapshot(); }
        if do_extreme_heat { self.publish_extreme_heat_snapshot(); }
    }

    fn publish_history_snapshot(&mut self) {
        let now_epoch = unsafe { esp_idf_svc::sys::time(std::ptr::null_mut()) } as i64;
        // Never overwrite the broker's good retained copy with an empty/unrecovered
        // snapshot. Two ways that can happen right after boot:
        //  * clock not yet NTP-synced — recovery is still pending (it waits for a
        //    valid clock), so RAM history hasn't been restored yet;
        //  * a retained snapshot existed but recovery hasn't applied it.
        // In both cases publishing now would wipe the persisted history. Hold off
        // until the clock is valid and either recovery has run or there was nothing
        // to recover.
        {
            let st = self.state.lock().unwrap();
            if now_epoch < 1_700_000_000 || st.pending_history_json.is_some() {
                return;
            }
        }
        let json = {
            let st = self.state.lock().unwrap();
            st.temp_history.snapshot_json(now_epoch)
        };
        if json.contains("\"points\":[]") {
            return;
        }
        self.pub_retained("persist/temp_history", &json);
    }

    fn publish_extreme_heat_snapshot(&mut self) {
        let json = {
            let st = self.state.lock().unwrap();
            let mut events = String::from("[");
            for (i, e) in st.eh_events.iter().enumerate() {
                if i > 0 { events.push(','); }
                events.push_str(&format!(
                    "{{\"epoch\":{},\"level\":{},\"reason\":\"{}\"}}",
                    e.epoch, e.level, e.reason.as_str()
                ));
            }
            events.push(']');
            format!(
                "{{\"enabled\":{},\"currentLevel\":{},\"lastChangeEpoch\":{},\"reason\":\"{}\",\"events\":{}}}",
                st.config.extreme_heat_enabled, st.eh_current_level, st.eh_last_change_epoch,
                st.eh_current_reason.as_str(), events
            )
        };
        self.pub_retained("persist/extreme_heat", &json);
    }

    fn publish_sensor_data(&mut self) {
        let st = self.state.lock().unwrap();
        let d = &st.cwl_data;

        // Collect all topic/payload pairs while holding the lock
        let mut msgs: Vec<(String, String)> = Vec::with_capacity(25);

        msgs.push(("ventilation/level".into(), d.ventilation_level.to_string()));
        msgs.push(("ventilation/level_name".into(), ventilation_level_name(d.ventilation_level).into()));
        msgs.push(("ventilation/relative".into(), d.relative_ventilation.to_string()));
        msgs.push(("temperature/supply".into(), format!("{:.1}", d.supply_temp)));
        msgs.push(("temperature/exhaust".into(), format!("{:.1}", d.exhaust_temp)));

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
        self.pub_retained("health/last_panic", &crate::panic_capture::last_panic().unwrap_or_default());

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
                // Humidity sensor topics are user-configured (not base-prefixed).
                if try_ingest_sensor(topic, data, state) {
                    return;
                }
                // Retained persistence snapshots: stash the raw bytes for the
                // main loop to parse (no heavy JSON work on this callback thread)
                // and apply only the FIRST one per topic after boot.
                if topic.ends_with("/persist/temp_history") {
                    let mut st = state.lock().unwrap();
                    if !st.history_recovered {
                        st.history_recovered = true;
                        st.pending_history_json = Some(data.to_vec());
                    }
                } else if topic.ends_with("/persist/extreme_heat") {
                    let mut st = state.lock().unwrap();
                    if !st.extreme_heat_recovered {
                        st.extreme_heat_recovered = true;
                        st.pending_extreme_heat_json = Some(data.to_vec());
                    }
                } else {
                    let msg = std::str::from_utf8(data).unwrap_or("");
                    handle_command(topic, msg, state, base_topic);
                }
            }
        }
        _ => {}
    }
}

/// If `topic` is a configured humidity sensor, parse humidity/temperature/
/// pressure and store the latest reading. Returns true if it matched a sensor.
fn try_ingest_sensor(topic: &str, data: &[u8], state: &AppState) -> bool {
    let mut st = state.lock().unwrap();
    let is_inside = st.config.humidity_inside_topics.iter().any(|t| t == topic);
    let is_outside = !st.config.humidity_outside_topic.is_empty()
        && st.config.humidity_outside_topic == topic;
    if !is_inside && !is_outside {
        return false;
    }
    // Sensor payloads are small — parsing on the callback thread is fine.
    let val: serde_json::Value = match serde_json::from_slice(data) {
        Ok(v) => v,
        Err(_) => return true, // matched a sensor topic; ignore malformed payload
    };
    let humidity = match val["humidity"].as_f64() {
        Some(h) => h as f32,
        None => return true, // humidity is required
    };
    let temperature = val["temperature"].as_f64().map(|v| v as f32);
    let pressure = val["pressure"].as_f64().map(|v| v as f32);
    let now_ms = unsafe { (esp_idf_svc::sys::esp_timer_get_time() / 1000) as u32 };
    let sample = HumiditySample { humidity, temperature, pressure, updated_ms: now_ms };
    if let Some(p) = pressure {
        st.ambient_pressure_kpa = pressure_to_kpa(p);
    }
    if is_inside {
        st.humidity_inside.insert(topic.to_string(), sample);
    }
    if is_outside {
        st.humidity_outside = Some(sample);
    }
    true
}

fn handle_command(topic: &str, message: &str, state: &AppState, _base_topic: &str) {
    let mut st = state.lock().unwrap();

    if topic.ends_with("/set/level") {
        if let Ok(level) = message.trim().parse::<u8>() {
            if level <= 3 {
                st.requested_vent_level = level;
                st.config.ventilation_level = level;
                st.initial_level_known = true;
                st.display_wake_requested = true;
                st.push_history_marker_now(level, crate::app_state::Reason::Manual);
                info!("MQTT: Level set to {} ({})", level, ventilation_level_name(level));
            }
        }
    } else if topic.ends_with("/set/bypass") {
        let open = matches!(message.trim(), "1" | "true" | "on");
        st.requested_bypass_open = open;
        st.config.bypass_open = open;
        st.persist_config = true;
        st.display_wake_requested = true;
        info!("MQTT: Bypass {}", if open { "open" } else { "closed" });
    } else if topic.ends_with("/set/filter_reset") {
        if matches!(message.trim(), "1" | "true") {
            st.requested_filter_reset = true;
            info!("MQTT: Filter reset triggered");
        }
    } else if topic.ends_with("/set/off_timer") {
        // Payload is minutes (15..=20160). Range covers the encoder table:
        // 15m through 2w.
        if let Ok(minutes) = message.trim().parse::<u16>() {
            if (15..=20160).contains(&minutes) {
                st.timed_off_request = Some(minutes);
                st.display_wake_requested = true;
                info!("MQTT: Timed off requested for {} min", minutes);
            }
        }
    }
}
