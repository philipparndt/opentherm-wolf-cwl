//! MQTT client — publish sensor data, subscribe to commands.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use esp_idf_svc::mqtt::client::{EspMqttClient, EspMqttEvent, EventPayload, MqttClientConfiguration, QoS};
use log::{info, warn};

use crate::app_state::{AppStateInner, HumiditySample};
use crate::cwl_data::ventilation_level_name;

type AppState = Arc<Mutex<AppStateInner>>;

const SENSOR_INTERVAL_MS: u32 = 11_000;
const HEALTH_INTERVAL_MS: u32 = 300_000;

/// Cadence for the command/acknowledgement block (`publish_command_state`).
///
/// A `set/*` command lands in `AppStateInner` immediately but only reaches the
/// unit on the next OpenTherm poll cycle, and is only confirmed a cycle later.
/// Publishing that block on the 11 s sensor tick meant an external consumer saw
/// *nothing* for up to 11 s after its own command — long enough for it to
/// conclude the command was lost. At 1 s the echo is effectively instant; the
/// change filter means an idle device still sends nothing.
const FAST_INTERVAL_MS: u32 = 1_000;

/// How often every state topic is republished even when nothing changed.
///
/// State topics are published change-only (see `pub_state`), which cuts the
/// message rate by roughly an order of magnitude. This periodic full pass is
/// the safety net: it refreshes the broker's retained copies (a broker restart
/// without persistence drops them) and gives time-series consumers a guaranteed
/// sample floor for values that sit still for hours.
const FULL_REFRESH_INTERVAL_MS: u32 = 900_000;

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
    /// Last payload published per state sub-topic. A tick only puts a message on
    /// the wire when the formatted payload actually differs, which is what keeps
    /// the ~35 retained topics from emitting ~3 messages/second around the clock.
    /// Formatting happens before the compare, so the printed precision (0.1 °C,
    /// 0.01 kJ/kg …) doubles as the deadband.
    published: HashMap<String, String>,
    /// Timestamp of the last unconditional full publish.
    last_full_ms: u32,
    /// Timestamp of the last command/acknowledgement publish.
    last_fast_ms: u32,
    /// When set, the next publish pass bypasses the change check and sends
    /// everything. Set at boot, on every broker reconnect, and every
    /// `FULL_REFRESH_INTERVAL_MS`.
    force_full: bool,
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

        // Buffers sized to hold the retained history snapshot (~16 KB of
        // downsampled timestamped points: temperature + humidity + enthalpy) in
        // a single frame, so neither the publish nor the recovery read fragments.
        let conf = MqttClientConfiguration {
            buffer_size: 20480,
            out_buffer_size: 20480,
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
            published: HashMap::new(),
            last_full_ms: 0,
            last_fast_ms: 0,
            force_full: true,
        })
    }

    /// Subscribe to any configured humidity-sensor topics not yet subscribed.
    /// Idempotent: called at setup and each tick so config edits apply without
    /// a reboot.
    fn sync_sensor_subscriptions(&mut self) {
        let topics: Vec<String> = {
            let st = self.state.lock().unwrap();
            let mut v = st.config.humidity_inside_topics.clone();
            v.extend(st.config.humidity_outside_topics.iter().cloned());
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
            // A reconnect may mean the broker restarted and lost every retained
            // message, so forget what we believe is on the broker and resend the
            // full state instead of only the deltas.
            self.published.clear();
            self.force_full = true;
        }
        self.last_connected = now_connected;

        // Pick up newly-configured sensor topics (cheap; no-op when unchanged).
        self.sync_sensor_subscriptions();

        if now_ms.wrapping_sub(self.last_full_ms) >= FULL_REFRESH_INTERVAL_MS {
            self.force_full = true;
        }

        if self.force_full || now_ms.wrapping_sub(self.last_fast_ms) >= FAST_INTERVAL_MS {
            self.last_fast_ms = now_ms;
            self.publish_command_state();
        }

        if self.force_full || now_ms.wrapping_sub(self.last_publish_ms) >= SENSOR_INTERVAL_MS {
            self.last_publish_ms = now_ms;
            self.publish_sensor_data();
        }

        if self.force_full || now_ms.wrapping_sub(self.last_health_ms) >= HEALTH_INTERVAL_MS {
            self.last_health_ms = now_ms;
            self.publish_health_data();
        }

        if self.force_full {
            self.force_full = false;
            self.last_full_ms = now_ms;
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
            // Compact bypass transitions `[[epoch,open01],…]`, restored on reboot.
            let mut bypass = String::from("[");
            for (i, e) in st.bypass_events.iter().enumerate() {
                if i > 0 { bypass.push(','); }
                bypass.push_str(&format!("[{},{}]", e.epoch, if e.open { 1 } else { 0 }));
            }
            bypass.push(']');
            st.temp_history.snapshot_json(now_epoch, &bypass)
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

    /// Command / acknowledgement state — everything a consumer needs to mirror
    /// the web UI's optimistic behaviour after issuing a `set/*` command:
    ///
    /// * `ventilation/requested` moves the instant the command is accepted,
    ///   before the unit has seen it (what the UI highlights right away),
    /// * `ventilation/actual` is the unit's real running level from ID 77
    ///   (what the UI waits for to clear its spinner),
    /// * `ventilation/pending` is the derived "not applied yet" flag.
    ///
    /// Published on the fast tick and change-filtered like everything else.
    fn publish_command_state(&mut self) {
        let mut msgs: Vec<(&'static str, String)> = Vec::with_capacity(16);
        {
            let st = self.state.lock().unwrap();
            let d = &st.cwl_data;

            // The level the device is driving toward. Every source funnels into
            // this field — MQTT set/level, the web API, the encoder, schedules
            // and extreme-heat mode — so the optimistic echo covers all of them,
            // not just MQTT-issued commands.
            let requested = st.requested_vent_level.min(3);
            // The level the unit is actually running, mapped from the reported
            // relative ventilation (ID 77) exactly like the UI's `actualLevel`.
            let actual = crate::cwl_data::VentLevel::from_relative_pct(d.relative_ventilation) as u8;

            msgs.push(("ventilation/requested", requested.to_string()));
            msgs.push(("ventilation/requested_name", ventilation_level_name(requested).into()));
            msgs.push(("ventilation/actual", actual.to_string()));
            msgs.push(("ventilation/pending", if requested != actual { "1" } else { "0" }.into()));
            // The raw ID 77 reading `actual` is derived from — same tick, so a
            // consumer watching either sees the confirmation at the same moment.
            msgs.push(("ventilation/relative", d.relative_ventilation.to_string()));
            // The unit's acknowledgement of our ID 71 write — kept for
            // compatibility; it trails `requested` by up to one poll cycle.
            msgs.push(("ventilation/level", d.ventilation_level.to_string()));
            msgs.push(("ventilation/level_name", ventilation_level_name(d.ventilation_level).into()));

            msgs.push(("status/connected", if d.connected { "1" } else { "0" }.into()));
            msgs.push(("status/filter", if d.filter_dirty { "1" } else { "0" }.into()));
            msgs.push(("status/bypass", if d.ventilation_active { "1" } else { "0" }.into()));

            // `bypass/mode` is the requested state (optimistic), `status/bypass`
            // the unit-reported one — same requested/actual split as the level.
            msgs.push(("bypass/mode", if st.requested_bypass_open { "summer" } else { "winter" }.into()));
            msgs.push(("bypass/pending",
                if st.requested_bypass_open != d.ventilation_active { "1" } else { "0" }.into()));

            msgs.push(("schedule/active", if st.schedule_active { "1" } else { "0" }.into()));
            msgs.push(("schedule/override", if st.schedule_override { "1" } else { "0" }.into()));
            msgs.push(("off_timer/active", if st.timed_off_active { "1" } else { "0" }.into()));
            msgs.push(("off_timer/remaining", st.timed_off_remaining_min.to_string()));
        }

        for (sub_topic, payload) in &msgs {
            self.pub_state(sub_topic, payload);
        }
    }

    fn publish_sensor_data(&mut self) {
        let st = self.state.lock().unwrap();
        let d = &st.cwl_data;

        // Collect all topic/payload pairs while holding the lock. Command state
        // (level, bypass, schedule, off-timer flags) is not here — it rides the
        // 1 s tick in publish_command_state() so commands echo promptly.
        let mut msgs: Vec<(String, String)> = Vec::with_capacity(25);

        msgs.push(("temperature/supply".into(), format!("{:.1}", d.supply_temp)));
        msgs.push(("temperature/exhaust".into(), format!("{:.1}", d.exhaust_temp)));

        if d.supports_id84 { msgs.push(("fan/exhaust_speed".into(), d.exhaust_fan_speed.to_string())); }
        if d.supports_id85 { msgs.push(("fan/supply_speed".into(), d.supply_fan_speed.to_string())); }

        if d.tsp_valid[52] { msgs.push(("airflow/current_volume".into(), d.current_volume.to_string())); }
        if d.tsp_valid[55] { msgs.push(("temperature/atmospheric".into(), d.temp_atmospheric.to_string())); }
        if d.tsp_valid[56] { msgs.push(("temperature/indoor".into(), d.temp_indoor.to_string())); }
        if d.tsp_valid[64] { msgs.push(("pressure/input_duct".into(), d.input_duct_pressure.to_string())); }
        if d.tsp_valid[66] { msgs.push(("pressure/output_duct".into(), d.output_duct_pressure.to_string())); }
        if d.tsp_valid[68] { msgs.push(("status/frost".into(), d.frost_status.to_string())); }

        msgs.push(("bypass/schedule_active".into(), "0".into()));
        msgs.push(("bypass/override".into(), "0".into()));

        if d.tsp_valid[54] { msgs.push(("status/bypass_position".into(), d.bypass_status.to_string())); }

        // Computed psychrometrics — the aggregated climate-decision inputs shown
        // on /api/status (lowest temperature, highest humidity per side, with
        // absolute humidity and specific enthalpy). Published as individual
        // retained `climate/*` topics so Telegraf & co. can scrape the subtree.
        // The ambient pressure feeding the calc is always published; the
        // indoor/outdoor blocks are skipped when no side has a fresh sensor
        // (inputs() is None), leaving any prior retained value untouched.
        let now_ms = unsafe { (esp_idf_svc::sys::esp_timer_get_time() / 1000) as u32 };
        msgs.push(("climate/ambient_pressure".into(), format!("{:.2}", st.ambient_pressure_kpa)));
        if let Some(inp) = crate::humidity::inputs(&st, now_ms, d.exhaust_temp, d.supply_temp) {
            for (side, air) in [("indoor", &inp.indoor), ("outdoor", &inp.outdoor)] {
                msgs.push((format!("climate/{}/temperature", side), format!("{:.1}", air.temp)));
                msgs.push((format!("climate/{}/rh", side), format!("{:.1}", air.rh)));
                msgs.push((format!("climate/{}/ah", side), format!("{:.2}", air.ah)));
                msgs.push((format!("climate/{}/enthalpy", side), format!("{:.2}", air.h)));
            }
        }

        drop(st); // Release lock before publishing

        for (sub_topic, payload) in &msgs {
            self.pub_state(sub_topic, payload);
        }
    }

    fn publish_health_data(&mut self) {
        let uptime = unsafe { esp_idf_svc::sys::esp_timer_get_time() / 1_000_000 };
        let free_heap = unsafe { esp_idf_svc::sys::esp_get_free_heap_size() };

        self.pub_state("health/uptime", &uptime.to_string());
        self.pub_state("health/free_heap", &free_heap.to_string());
        self.pub_state("health/reboot_reason", crate::watchdog::reboot_reason());
        self.pub_state("health/crash_count", "0");
        self.pub_state("health/last_panic", &crate::panic_capture::last_panic().unwrap_or_default());

        let ot_age = {
            let st = self.state.lock().unwrap();
            let now_ms = unsafe { (esp_idf_svc::sys::esp_timer_get_time() / 1000) as u32 };
            if st.cwl_data.last_response_ms > 0 {
                now_ms.wrapping_sub(st.cwl_data.last_response_ms)
            } else {
                0
            }
        };
        self.pub_state("health/ot_response_age", &ot_age.to_string());
    }

    /// Publish a state topic, but only if its payload changed since the last
    /// time we sent it (or a full refresh is due). Retained delivery means a
    /// late subscriber still gets the current value immediately, so skipping
    /// unchanged repeats costs consumers nothing.
    fn pub_state(&mut self, sub_topic: &str, payload: &str) {
        if !self.force_full {
            if self.published.get(sub_topic).map(|p| p == payload).unwrap_or(false) {
                return;
            }
        }
        self.pub_retained(sub_topic, payload);
        match self.published.get_mut(sub_topic) {
            Some(prev) => {
                prev.clear();
                prev.push_str(payload);
            }
            None => {
                self.published.insert(sub_topic.to_string(), payload.to_string());
            }
        }
    }

    /// Publish unconditionally (bridge markers and the retained `persist/*`
    /// snapshots, which are already emitted only on demand).
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
    let is_outside = st.config.humidity_outside_topics.iter().any(|t| t == topic);
    if !is_inside && !is_outside {
        return false;
    }
    // Sensor payloads are small — parsing on the callback thread is fine.
    let val: serde_json::Value = match serde_json::from_slice(data) {
        Ok(v) => v,
        Err(_) => return true, // matched a sensor topic; ignore malformed payload
    };
    let raw_humidity = val["humidity"].as_f64().map(|v| v as f32);
    let raw_temperature = val["temperature"].as_f64().map(|v| v as f32);
    let pressure = val["pressure"].as_f64().map(|v| v as f32);
    if raw_humidity.is_none() && raw_temperature.is_none() {
        return true; // matched a sensor topic but carries no usable reading
    }
    let now_ms = unsafe { (esp_idf_svc::sys::esp_timer_get_time() / 1000) as u32 };

    use crate::sensor_filter::{self, FieldFilter, in_range};
    use crate::sensor_filter::{HUMIDITY_MAX_STEP, HUMIDITY_MIN, HUMIDITY_MAX};
    use crate::sensor_filter::{TEMPERATURE_MAX_STEP, TEMPERATURE_MIN, TEMPERATURE_MAX};

    // Carry this sensor's previous filter state so the spike guard can compare
    // the new reading against its recent history.
    let prev = st.humidity_inside.get(topic)
        .or_else(|| st.humidity_outside.get(topic))
        .copied();
    let mut hum_filter = prev.map_or_else(FieldFilter::default, |s| s.hum_filter);
    let mut temp_filter = prev.map_or_else(FieldFilter::default, |s| s.temp_filter);

    // Filter each present field; a missing field carries the last trusted value.
    let humidity = match raw_humidity {
        Some(h) => hum_filter.update(h, HUMIDITY_MAX_STEP, HUMIDITY_MIN, HUMIDITY_MAX),
        None => prev.and_then(|s| s.humidity),
    };
    let temperature = match raw_temperature {
        Some(t) => temp_filter.update(t, TEMPERATURE_MAX_STEP, TEMPERATURE_MIN, TEMPERATURE_MAX),
        None => prev.and_then(|s| s.temperature),
    };

    // Freshness only advances when the sensor actually sent a physically-plausible
    // value, so a sensor emitting only garbage ages out of the decision instead
    // of freezing a held value as "fresh" forever.
    let usable = raw_humidity.map_or(false, |h| in_range(h, HUMIDITY_MIN, HUMIDITY_MAX))
        || raw_temperature.map_or(false, |t| in_range(t, TEMPERATURE_MIN, TEMPERATURE_MAX));
    let updated_ms = match prev {
        Some(p) if !usable => p.updated_ms,
        _ => now_ms,
    };

    let sample = HumiditySample { humidity, temperature, pressure, updated_ms, hum_filter, temp_filter };

    // Only adopt a physically-plausible ambient pressure; ignore zero/garbage so
    // a bad pressure read can't skew the enthalpy calculation.
    if let Some(p) = pressure {
        let kpa = pressure_to_kpa(p);
        if sensor_filter::in_range(kpa, 80.0, 110.0) {
            st.ambient_pressure_kpa = kpa;
        }
    }
    if is_inside {
        st.humidity_inside.insert(topic.to_string(), sample);
    }
    if is_outside {
        st.humidity_outside.insert(topic.to_string(), sample);
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
        st.set_bypass_open(open);
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
        //
        // `0` (and the word forms) cancels a running timer — the MQTT
        // equivalent of the UI's Cancel button, which posts to
        // /api/off_timer/cancel. Without this a client could start timed-off
        // mode over MQTT but had no way to end it: 0 parsed fine, then fell out
        // of the range check and was dropped in silence.
        let msg = message.trim();
        if matches!(msg, "0" | "off" | "false" | "cancel") {
            st.cancel_timed_off = true;
            st.display_wake_requested = true;
            info!("MQTT: Timed off cancelled");
        } else if let Ok(minutes) = msg.parse::<u16>() {
            if (15..=20160).contains(&minutes) {
                st.timed_off_request = Some(minutes);
                st.display_wake_requested = true;
                info!("MQTT: Timed off requested for {} min", minutes);
            } else {
                warn!("MQTT: Ignoring off_timer={} (expected 0 to cancel, or 15..=20160)", minutes);
            }
        } else {
            warn!("MQTT: Ignoring unparseable off_timer payload {:?}", msg);
        }
    }
}
