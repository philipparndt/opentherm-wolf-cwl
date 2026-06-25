//! HTTP server — REST API and static file serving.

use std::sync::{Arc, Mutex};

use esp_idf_svc::http::server::{Configuration, EspHttpConnection, EspHttpServer};
use esp_idf_svc::http::Method;
use esp_idf_svc::io::{EspIOError, Write};
use log::info;
use base64::Engine;
use serde_json::json;

use crate::app_state::AppStateInner;
use crate::cwl_data::ventilation_level_name;
use crate::i18n::Language;

type AppState = Arc<Mutex<AppStateInner>>;
type HandlerResult = Result<(), EspIOError>;

/// Check if request has a valid auth cookie.
fn is_authenticated(req: &esp_idf_svc::http::server::Request<&mut EspHttpConnection>) -> bool {
    req.header("Cookie")
        .map(|c| c.contains("auth_token="))
        .unwrap_or(false)
}

/// Read request body into a Vec<u8>.
fn read_body(req: &mut esp_idf_svc::http::server::Request<&mut EspHttpConnection>) -> Vec<u8> {
    let mut buf = vec![0u8; 2048];
    let mut body = Vec::new();
    loop {
        match req.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => body.extend_from_slice(&buf[..n]),
            Err(_) => break,
        }
    }
    body
}

fn send_unauthorized(req: esp_idf_svc::http::server::Request<&mut EspHttpConnection>) -> HandlerResult {
    let mut resp = req.into_response(401, None, &[])?;
    resp.write_all(b"Unauthorized")?;
    Ok(())
}

fn send_ok(req: esp_idf_svc::http::server::Request<&mut EspHttpConnection>) -> HandlerResult {
    let mut resp = req.into_response(200, None, &[("Content-Type", "application/json")])?;
    resp.write_all(b"{\"success\":true}")?;
    Ok(())
}

fn send_json_body(req: esp_idf_svc::http::server::Request<&mut EspHttpConnection>, body: &str) -> HandlerResult {
    let mut resp = req.into_response(200, None, &[("Content-Type", "application/json")])?;
    resp.write_all(body.as_bytes())?;
    Ok(())
}

fn content_type(path: &str) -> &'static str {
    if path.ends_with(".js") { "application/javascript" }
    else if path.ends_with(".css") { "text/css" }
    else if path.ends_with(".html") { "text/html" }
    else if path.ends_with(".ico") { "image/x-icon" }
    else { "application/octet-stream" }
}

/// The web bundle ships with fixed filenames (app.js / style.css / index.html),
/// not content-hashed ones, so they MUST revalidate after a filesystem OTA —
/// otherwise the browser serves a stale app.js for up to the max-age and the
/// new UI only shows after a force-reload. `no-cache` lets the browser keep a
/// copy but forces a re-check on every load (we have no ETag, so it refetches).
/// Immutable assets like the favicon can still be cached for a while.
fn cache_control(path: &str) -> &'static str {
    if path.ends_with(".html") || path.ends_with(".js") || path.ends_with(".css") {
        "no-cache"
    } else {
        "public, max-age=3600"
    }
}

fn serve_file(req: esp_idf_svc::http::server::Request<&mut EspHttpConnection>, path: &str) -> HandlerResult {
    match std::fs::read(path) {
        Ok(data) => {
            let ct = content_type(path);
            let mut resp = req.into_response(200, None, &[
                ("Content-Type", ct),
                ("Cache-Control", cache_control(path)),
            ])?;
            resp.write_all(&data)?;
            Ok(())
        }
        Err(_) => {
            // SPA fallback: try index.html
            if path != "/www/index.html" {
                if let Ok(data) = std::fs::read("/www/index.html") {
                    let mut resp = req.into_response(200, None, &[
                        ("Content-Type", "text/html"),
                        ("Cache-Control", "no-cache"),
                    ])?;
                    resp.write_all(&data)?;
                    return Ok(());
                }
            }
            let mut resp = req.into_response(404, None, &[("Content-Type", "text/plain")])?;
            resp.write_all(b"Not found")?;
            Ok(())
        }
    }
}

pub fn start_server(state: AppState) -> Result<EspHttpServer<'static>, EspIOError> {
    let mut conf = Configuration::default();
    conf.http_port = 80;
    conf.stack_size = 8192;

    let mut server = EspHttpServer::new(&conf)?;

    // --- POST /api/login ---
    let s = state.clone();
    server.fn_handler("/api/login", Method::Post, move |mut req| -> HandlerResult {
        let body = read_body(&mut req);
        if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&body) {
            let username = val["username"].as_str().unwrap_or("");
            let password = val["password"].as_str().unwrap_or("");
            let st = s.lock().unwrap();
            if username == st.config.web_username && password == st.config.web_password {
                let mut resp = req.into_response(200, None, &[
                    ("Content-Type", "application/json"),
                    ("Set-Cookie", "auth_token=valid; Path=/; HttpOnly; Max-Age=86400"),
                ])?;
                resp.write_all(b"{\"success\":true}")?;
                return Ok(());
            }
        }
        let mut resp = req.into_response(401, None, &[("Content-Type", "application/json")])?;
        resp.write_all(b"{\"error\":\"Invalid credentials\"}")?;
        Ok(())
    })?;

    // --- GET /api/status ---
    let s = state.clone();
    server.fn_handler("/api/status", Method::Get, move |req| -> HandlerResult {
        if !is_authenticated(&req) {
            return send_unauthorized(req);
        }
        let st = s.lock().unwrap();
        let d = &st.cwl_data;

        // Humidity sensors + derived psychrometrics for the decision explainer.
        let now_ms = unsafe { (esp_idf_svc::sys::esp_timer_get_time() / 1000) as u32 };
        let inp = crate::humidity::inputs(
            &st, now_ms, st.cwl_data.exhaust_temp, st.cwl_data.supply_temp);
        let mut sensors: Vec<serde_json::Value> = Vec::new();
        for (topic, sm) in st.humidity_inside.iter() {
            sensors.push(json!({
                "topic": topic, "role": "indoor",
                "humidity": sm.humidity, "temperature": sm.temperature, "pressure": sm.pressure,
                "fresh": crate::humidity::is_fresh(sm.updated_ms, now_ms),
            }));
        }
        if let Some(sm) = st.humidity_outside.as_ref() {
            sensors.push(json!({
                "topic": st.config.humidity_outside_topic, "role": "outdoor",
                "humidity": sm.humidity, "temperature": sm.temperature, "pressure": sm.pressure,
                "fresh": crate::humidity::is_fresh(sm.updated_ms, now_ms),
            }));
        }
        let hum_json = json!({
            "active": inp.is_some(),
            "ambientPressureKpa": st.ambient_pressure_kpa,
            "indoorRh": inp.as_ref().map(|i| i.indoor.rh),
            "outdoorRh": inp.as_ref().map(|i| i.outdoor.rh),
            "indoorAh": inp.as_ref().map(|i| i.indoor.ah),
            "outdoorAh": inp.as_ref().map(|i| i.outdoor.ah),
            "indoorEnthalpy": inp.as_ref().map(|i| i.indoor.h),
            "outdoorEnthalpy": inp.as_ref().map(|i| i.outdoor.h),
            "sensors": sensors,
        });

        let body = json!({
            "ventilation": {
                "level": d.ventilation_level,
                "levelName": ventilation_level_name(d.ventilation_level),
                "requestedLevel": st.requested_vent_level,
                "relative": d.relative_ventilation,
                "scheduleActive": st.schedule_active,
                "override": st.schedule_override,
            },
            "temperature": {
                "supply": d.supply_temp,
                "exhaust": d.exhaust_temp,
            },
            "status": {
                "filter": d.filter_dirty,
                "bypass": d.ventilation_active,
                "connected": d.connected,
            },
            "system": {
                "uptime": unsafe { esp_idf_svc::sys::esp_timer_get_time() / 1_000_000 },
                "freeHeap": unsafe { esp_idf_svc::sys::esp_get_free_heap_size() },
                "version": env!("CARGO_PKG_VERSION"),
                "mqttConnected": st.mqtt_connected,
                "wifiRssi": 0,
                "simulated": st.simulated,
                "lastPanic": crate::panic_capture::last_panic(),
            },
            "timedOff": {
                "active": st.timed_off_active,
                "remainingMinutes": st.timed_off_remaining_min,
            },
            "extremeHeat": {
                "enabled": st.config.extreme_heat_enabled,
                "currentLevel": st.eh_current_level,
                "lastChangeEpoch": st.eh_last_change_epoch,
                "reason": st.eh_current_reason.as_str(),
                "protectionEnabled": st.config.humidity_protection_enabled,
                "protectionActive": st.protection_active,
            },
            "humidity": hum_json,
            "airflow": {
                "reduced": if d.tsp_valid[0] && d.tsp_valid[1] { (d.tsp_values[0] as u32) | ((d.tsp_values[1] as u32) << 8) } else { 100 },
                "normal": if d.tsp_valid[2] && d.tsp_valid[3] { (d.tsp_values[2] as u32) | ((d.tsp_values[3] as u32) << 8) } else { 130 },
                "party": if d.tsp_valid[4] && d.tsp_valid[5] { (d.tsp_values[4] as u32) | ((d.tsp_values[5] as u32) << 8) } else { 195 },
            },
        });
        send_json_body(req, &body.to_string())
    })?;

    // --- POST /api/ventilation/level ---
    let s = state.clone();
    server.fn_handler("/api/ventilation/level", Method::Post, move |mut req| -> HandlerResult {
        if !is_authenticated(&req) {
            return send_unauthorized(req);
        }
        let body = read_body(&mut req);
        if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&body) {
            if let Some(level) = val["level"].as_i64() {
                if (0..=3).contains(&level) {
                    let mut st = s.lock().unwrap();
                    st.requested_vent_level = level as u8;
                    st.config.ventilation_level = level as u8;
                    st.schedule_override = true;
                    st.initial_level_known = true;
                    st.push_history_marker_now(level as u8, crate::app_state::Reason::Manual);
                    return send_ok(req);
                }
            }
        }
        let mut resp = req.into_response(400, None, &[("Content-Type", "application/json")])?;
        resp.write_all(b"{\"error\":\"Invalid level (0-3)\"}")?;
        Ok(())
    })?;

    // --- POST /api/ventilation/resume ---
    let s = state.clone();
    server.fn_handler("/api/ventilation/resume", Method::Post, move |req| -> HandlerResult {
        if !is_authenticated(&req) {
            return send_unauthorized(req);
        }
        s.lock().unwrap().schedule_override = false;
        send_ok(req)
    })?;

    // --- POST /api/encoder ---
    let s = state.clone();
    server.fn_handler("/api/encoder", Method::Post, move |mut req| -> HandlerResult {
        if !is_authenticated(&req) {
            return send_unauthorized(req);
        }
        let body = read_body(&mut req);
        if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&body) {
            if let Some(action) = val["action"].as_str() {
                s.lock().unwrap().encoder_action = Some(action.to_string());
            }
        }
        send_ok(req)
    })?;

    // --- POST /api/off_timer/cancel ---
    let s = state.clone();
    server.fn_handler("/api/off_timer/cancel", Method::Post, move |req| -> HandlerResult {
        if !is_authenticated(&req) { return send_unauthorized(req); }
        s.lock().unwrap().cancel_timed_off = true;
        send_ok(req)
    })?;

    // --- GET /api/config ---
    let s = state.clone();
    server.fn_handler("/api/config", Method::Get, move |req| -> HandlerResult {
        if !is_authenticated(&req) {
            return send_unauthorized(req);
        }
        let st = s.lock().unwrap();
        let c = &st.config;
        let body = json!({
            "network": {
                "wifiSsid": c.wifi_ssid,
                "wifiPassword": "********",
            },
            "mqtt": {
                "enabled": c.mqtt_enabled,
                "server": c.mqtt_server,
                "port": c.mqtt_port,
                "topic": c.mqtt_topic,
                "authEnabled": c.mqtt_auth_enabled,
                "username": c.mqtt_username,
                "password": "********",
            },
            "web": {
                "username": c.web_username,
                "password": "********",
            },
            "pins": {
                "otIn": c.ot_in_pin,
                "otOut": c.ot_out_pin,
                "sda": c.sda_pin,
                "scl": c.scl_pin,
                "encClk": c.enc_clk_pin,
                "encDt": c.enc_dt_pin,
                "encSw": c.enc_sw_pin,
            },
            "extremeHeat": {
                "enabled": c.extreme_heat_enabled,
            },
            "humidity": {
                "insideTopics": c.humidity_inside_topics,
                "outsideTopic": c.humidity_outside_topic,
                "protectionEnabled": c.humidity_protection_enabled,
            },
            "configured": c.configured,
            "language": c.language.code(),
        });
        send_json_body(req, &body.to_string())
    })?;

    // --- POST /api/config ---
    let s = state.clone();
    server.fn_handler("/api/config", Method::Post, move |mut req| -> HandlerResult {
        if !is_authenticated(&req) {
            return send_unauthorized(req);
        }
        let body = read_body(&mut req);
        if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&body) {
            let mut st = s.lock().unwrap();
            // Network
            if let Some(net) = val.get("network") {
                if let Some(v) = net["wifiSsid"].as_str() { st.config.wifi_ssid = v.to_string(); }
                if let Some(v) = net["wifiPassword"].as_str() {
                    if v != "********" { st.config.wifi_password = v.to_string(); }
                }
            }
            // MQTT
            if let Some(mqtt) = val.get("mqtt") {
                if let Some(v) = mqtt["enabled"].as_bool() { st.config.mqtt_enabled = v; }
                if let Some(v) = mqtt["server"].as_str() { st.config.mqtt_server = v.to_string(); }
                if let Some(v) = mqtt["port"].as_u64() { st.config.mqtt_port = v as u16; }
                if let Some(v) = mqtt["topic"].as_str() { st.config.mqtt_topic = v.to_string(); }
                if let Some(v) = mqtt["authEnabled"].as_bool() { st.config.mqtt_auth_enabled = v; }
                if let Some(v) = mqtt["username"].as_str() { st.config.mqtt_username = v.to_string(); }
                if let Some(v) = mqtt["password"].as_str() {
                    if v != "********" { st.config.mqtt_password = v.to_string(); }
                }
            }
            // Web
            if let Some(web) = val.get("web") {
                if let Some(v) = web["username"].as_str() { st.config.web_username = v.to_string(); }
                if let Some(v) = web["password"].as_str() {
                    if v != "********" { st.config.web_password = v.to_string(); }
                }
            }
            // Pins
            if let Some(pins) = val.get("pins") {
                if let Some(v) = pins["otIn"].as_u64() { st.config.ot_in_pin = v as u8; }
                if let Some(v) = pins["otOut"].as_u64() { st.config.ot_out_pin = v as u8; }
                if let Some(v) = pins["sda"].as_u64() { st.config.sda_pin = v as u8; }
                if let Some(v) = pins["scl"].as_u64() { st.config.scl_pin = v as u8; }
                if let Some(v) = pins["encClk"].as_u64() { st.config.enc_clk_pin = v as u8; }
                if let Some(v) = pins["encDt"].as_u64() { st.config.enc_dt_pin = v as u8; }
                if let Some(v) = pins["encSw"].as_u64() { st.config.enc_sw_pin = v as u8; }
            }
            // Extreme-heat mode toggle — persist so it survives reboot.
            if let Some(eh) = val.get("extremeHeat") {
                if let Some(v) = eh["enabled"].as_bool() {
                    st.config.extreme_heat_enabled = v;
                    st.persist_config = true;
                }
            }
            // Humidity sensors + moisture-protection toggle.
            if let Some(h) = val.get("humidity") {
                if let Some(arr) = h["insideTopics"].as_array() {
                    st.config.humidity_inside_topics =
                        arr.iter().filter_map(|t| t.as_str().map(|s| s.to_string())).collect();
                    st.persist_config = true;
                }
                if let Some(v) = h["outsideTopic"].as_str() {
                    st.config.humidity_outside_topic = v.to_string();
                    st.persist_config = true;
                }
                if let Some(v) = h["protectionEnabled"].as_bool() {
                    st.config.humidity_protection_enabled = v;
                    st.persist_config = true;
                }
            }
            if let Some(v) = val["configured"].as_bool() { st.config.configured = v; }
            if let Some(v) = val["language"].as_str() { st.config.language = Language::from_code(v); }
            info!("Config updated via POST /api/config");
            drop(st);
            return send_ok(req);
        }
        let mut resp = req.into_response(400, None, &[("Content-Type", "application/json")])?;
        resp.write_all(b"{\"error\":\"Invalid JSON\"}")?;
        Ok(())
    })?;

    // --- GET /api/backup (export all settings + schedules as JSON) ---
    let s = state.clone();
    server.fn_handler("/api/backup", Method::Get, move |req| -> HandlerResult {
        if !is_authenticated(&req) { return send_unauthorized(req); }
        let st = s.lock().unwrap();
        let c = &st.config;
        let body = json!({
            "config": {
                "network": {
                    "wifiSsid": c.wifi_ssid,
                    "wifiPassword": c.wifi_password,
                },
                "mqtt": {
                    "enabled": c.mqtt_enabled,
                    "server": c.mqtt_server,
                    "port": c.mqtt_port,
                    "topic": c.mqtt_topic,
                    "authEnabled": c.mqtt_auth_enabled,
                    "username": c.mqtt_username,
                    "password": c.mqtt_password,
                },
                "web": {
                    "username": c.web_username,
                    "password": c.web_password,
                },
                "pins": {
                    "otIn": c.ot_in_pin,
                    "otOut": c.ot_out_pin,
                    "sda": c.sda_pin,
                    "scl": c.scl_pin,
                    "encClk": c.enc_clk_pin,
                    "encDt": c.enc_dt_pin,
                    "encSw": c.enc_sw_pin,
                },
                "extremeHeat": {
                    "enabled": c.extreme_heat_enabled,
                },
                "humidity": {
                    "insideTopics": c.humidity_inside_topics,
                    "outsideTopic": c.humidity_outside_topic,
                    "protectionEnabled": c.humidity_protection_enabled,
                },
                "configured": c.configured,
            },
            "schedules": st.schedules,
            "bypassSchedule": st.bypass_schedule,
        });
        let json_str = body.to_string();
        let mut resp = req.into_response(200, None, &[
            ("Content-Type", "application/json"),
            ("Content-Disposition", "attachment; filename=\"wolf-cwl-backup.json\""),
        ])?;
        resp.write_all(json_str.as_bytes())?;
        Ok(())
    })?;

    // --- POST /api/restore (import all settings + schedules from JSON) ---
    let s = state.clone();
    server.fn_handler("/api/restore", Method::Post, move |mut req| -> HandlerResult {
        if !is_authenticated(&req) { return send_unauthorized(req); }
        let body = read_body(&mut req);
        if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&body) {
            let mut st = s.lock().unwrap();

            // Restore config
            if let Some(cfg) = val.get("config") {
                if let Some(net) = cfg.get("network") {
                    if let Some(v) = net["wifiSsid"].as_str() { st.config.wifi_ssid = v.to_string(); }
                    if let Some(v) = net["wifiPassword"].as_str() { st.config.wifi_password = v.to_string(); }
                }
                if let Some(mqtt) = cfg.get("mqtt") {
                    if let Some(v) = mqtt["enabled"].as_bool() { st.config.mqtt_enabled = v; }
                    if let Some(v) = mqtt["server"].as_str() { st.config.mqtt_server = v.to_string(); }
                    if let Some(v) = mqtt["port"].as_u64() { st.config.mqtt_port = v as u16; }
                    if let Some(v) = mqtt["topic"].as_str() { st.config.mqtt_topic = v.to_string(); }
                    if let Some(v) = mqtt["authEnabled"].as_bool() { st.config.mqtt_auth_enabled = v; }
                    if let Some(v) = mqtt["username"].as_str() { st.config.mqtt_username = v.to_string(); }
                    if let Some(v) = mqtt["password"].as_str() { st.config.mqtt_password = v.to_string(); }
                }
                if let Some(web) = cfg.get("web") {
                    if let Some(v) = web["username"].as_str() { st.config.web_username = v.to_string(); }
                    if let Some(v) = web["password"].as_str() { st.config.web_password = v.to_string(); }
                }
                if let Some(pins) = cfg.get("pins") {
                    if let Some(v) = pins["otIn"].as_u64() { st.config.ot_in_pin = v as u8; }
                    if let Some(v) = pins["otOut"].as_u64() { st.config.ot_out_pin = v as u8; }
                    if let Some(v) = pins["sda"].as_u64() { st.config.sda_pin = v as u8; }
                    if let Some(v) = pins["scl"].as_u64() { st.config.scl_pin = v as u8; }
                    if let Some(v) = pins["encClk"].as_u64() { st.config.enc_clk_pin = v as u8; }
                    if let Some(v) = pins["encDt"].as_u64() { st.config.enc_dt_pin = v as u8; }
                    if let Some(v) = pins["encSw"].as_u64() { st.config.enc_sw_pin = v as u8; }
                }
                if let Some(eh) = cfg.get("extremeHeat") {
                    if let Some(v) = eh["enabled"].as_bool() { st.config.extreme_heat_enabled = v; }
                }
                if let Some(h) = cfg.get("humidity") {
                    if let Some(arr) = h["insideTopics"].as_array() {
                        st.config.humidity_inside_topics =
                            arr.iter().filter_map(|t| t.as_str().map(|s| s.to_string())).collect();
                    }
                    if let Some(v) = h["outsideTopic"].as_str() { st.config.humidity_outside_topic = v.to_string(); }
                    if let Some(v) = h["protectionEnabled"].as_bool() { st.config.humidity_protection_enabled = v; }
                }
                st.config.configured = true;
            }

            // Restore schedules
            if let Some(sched) = val.get("schedules") {
                if let Ok(entries) = serde_json::from_value::<Vec<crate::scheduler::ScheduleEntry>>(sched.clone()) {
                    st.schedules = entries;
                }
            }

            // Restore bypass schedule
            if let Some(bp) = val.get("bypassSchedule") {
                if let Ok(schedule) = serde_json::from_value::<crate::scheduler::BypassSchedule>(bp.clone()) {
                    st.bypass_schedule = schedule;
                }
            }

            st.persist_schedules = true;
            st.persist_config = true;
            info!("Settings restored from backup");
            drop(st);
            return send_ok(req);
        }
        let mut resp = req.into_response(400, None, &[("Content-Type", "application/json")])?;
        resp.write_all(b"{\"error\":\"Invalid JSON\"}")?;
        Ok(())
    })?;

    // --- GET /api/schedules ---
    let s = state.clone();
    server.fn_handler("/api/schedules", Method::Get, move |req| -> HandlerResult {
        if !is_authenticated(&req) { return send_unauthorized(req); }
        let st = s.lock().unwrap();
        let json_str = serde_json::to_string(&st.schedules).unwrap_or_else(|_| "[]".into());
        send_json_body(req, &json_str)
    })?;

    // --- POST /api/schedules ---
    let s = state.clone();
    server.fn_handler("/api/schedules", Method::Post, move |mut req| -> HandlerResult {
        if !is_authenticated(&req) { return send_unauthorized(req); }
        let body = read_body(&mut req);
        if let Ok(entries) = serde_json::from_slice::<Vec<crate::scheduler::ScheduleEntry>>(&body) {
            let mut st = s.lock().unwrap();
            st.schedules = entries;
            st.persist_schedules = true;
            return send_ok(req);
        }
        let mut resp = req.into_response(400, None, &[("Content-Type", "application/json")])?;
        resp.write_all(b"{\"error\":\"Invalid schedule data\"}")?;
        Ok(())
    })?;

    // --- GET /api/bypass-schedule ---
    let s = state.clone();
    server.fn_handler("/api/bypass-schedule", Method::Get, move |req| -> HandlerResult {
        if !is_authenticated(&req) { return send_unauthorized(req); }
        let st = s.lock().unwrap();
        let json_str = serde_json::to_string(&st.bypass_schedule).unwrap_or_else(|_| "{}".into());
        send_json_body(req, &json_str)
    })?;

    // --- POST /api/bypass-schedule ---
    let s = state.clone();
    server.fn_handler("/api/bypass-schedule", Method::Post, move |mut req| -> HandlerResult {
        if !is_authenticated(&req) { return send_unauthorized(req); }
        let body = read_body(&mut req);
        if let Ok(schedule) = serde_json::from_slice::<crate::scheduler::BypassSchedule>(&body) {
            let mut st = s.lock().unwrap();
            st.bypass_schedule = schedule;
            st.persist_schedules = true;
            return send_ok(req);
        }
        let mut resp = req.into_response(400, None, &[("Content-Type", "application/json")])?;
        resp.write_all(b"{\"error\":\"Invalid bypass schedule\"}")?;
        Ok(())
    })?;

    // --- POST /api/ota/upload (firmware OTA) ---
    let s = state.clone();
    server.fn_handler("/api/ota/upload", Method::Post, move |mut req| -> HandlerResult {
        if !is_authenticated(&req) { return send_unauthorized(req); }
        use esp_idf_svc::sys::*;

        // Flush RAM-only state (temperature history + extreme-heat markers) to
        // the broker (retained) before we reboot into the new image. The MQTT
        // thread polls these flags every 100 ms and the upload below streams for
        // several seconds, so the freshest snapshot is persisted well before
        // esp_restart() — letting the new firmware recover it on boot instead of
        // starting empty.
        {
            let mut st = s.lock().unwrap();
            st.mqtt_publish_history = true;
            st.mqtt_publish_extreme_heat = true;
        }

        let mut ota_handle: esp_ota_handle_t = 0;
        let update_partition = unsafe { esp_ota_get_next_update_partition(std::ptr::null()) };
        if update_partition.is_null() {
            let mut resp = req.into_response(500, None, &[("Content-Type", "application/json")])?;
            resp.write_all(b"{\"error\":\"No OTA partition\"}")?;
            return Ok(());
        }

        let ret = unsafe { esp_ota_begin(update_partition, OTA_SIZE_UNKNOWN as usize, &mut ota_handle) };
        if ret != ESP_OK {
            let mut resp = req.into_response(500, None, &[("Content-Type", "application/json")])?;
            resp.write_all(b"{\"error\":\"OTA begin failed\"}")?;
            return Ok(());
        }

        info!("OTA: Firmware upload started");
        let mut buf = [0u8; 1024];
        let mut total: usize = 0;
        loop {
            match req.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    unsafe { esp_ota_write(ota_handle, buf.as_ptr() as *const _, n) };
                    total += n;
                }
                Err(_) => break,
            }
        }

        let ret = unsafe { esp_ota_end(ota_handle) };
        if ret == ESP_OK {
            unsafe { esp_ota_set_boot_partition(update_partition) };
            info!("OTA: Success ({} bytes), rebooting...", total);
            let mut resp = req.into_response(200, None, &[("Content-Type", "application/json")])?;
            resp.write_all(b"{\"success\":true}")?;
            // Reboot after response is sent
            std::thread::spawn(|| {
                std::thread::sleep(std::time::Duration::from_secs(1));
                unsafe { esp_restart() };
            });
            Ok(())
        } else {
            info!("OTA: Failed (err={})", ret);
            let mut resp = req.into_response(500, None, &[("Content-Type", "application/json")])?;
            resp.write_all(b"{\"error\":\"OTA verification failed\"}")?;
            Ok(())
        }
    })?;

    // --- POST /api/ota/fs (filesystem OTA — rewrites the SPIFFS web-UI partition) ---
    // The app-OTA handler above only touches the app partition; the web UI lives
    // in the separate "spiffs" partition, so this endpoint streams a new SPIFFS
    // image straight into it. It does NOT reboot — the caller is expected to push
    // the firmware image afterwards (which reboots, remounting the new filesystem).
    server.fn_handler("/api/ota/fs", Method::Post, move |mut req| -> HandlerResult {
        if !is_authenticated(&req) { return send_unauthorized(req); }
        use esp_idf_svc::sys::*;

        // Locate the SPIFFS partition (label "spiffs", matching partitions.csv).
        let label = b"spiffs\0";
        let part = unsafe {
            esp_partition_find_first(
                esp_partition_type_t_ESP_PARTITION_TYPE_DATA,
                esp_partition_subtype_t_ESP_PARTITION_SUBTYPE_DATA_SPIFFS,
                label.as_ptr() as *const _,
            )
        };
        if part.is_null() {
            let mut resp = req.into_response(500, None, &[("Content-Type", "application/json")])?;
            resp.write_all(b"{\"error\":\"No SPIFFS partition\"}")?;
            return Ok(());
        }
        let psize = unsafe { (*part).size } as usize;

        // Unmount the live filesystem before rewriting it (best effort — it may
        // already be unmounted). The web UI is unavailable until the device
        // reboots, which the firmware OTA step that follows is expected to do.
        unsafe { esp_vfs_spiffs_unregister(label.as_ptr() as *const _); }

        let ret = unsafe { esp_partition_erase_range(part, 0, psize) };
        if ret != ESP_OK {
            let mut resp = req.into_response(500, None, &[("Content-Type", "application/json")])?;
            resp.write_all(b"{\"error\":\"SPIFFS erase failed\"}")?;
            return Ok(());
        }

        info!("OTA-FS: SPIFFS upload started ({} byte partition)", psize);
        // Flash writes must be 4-byte aligned, so buffer into 4 KB blocks and
        // only program whole blocks; pad the final short block to a word boundary.
        let mut block = vec![0u8; 4096];
        let mut fill: usize = 0;
        let mut offset: usize = 0;
        let mut overflow = false;
        loop {
            match req.read(&mut block[fill..]) {
                Ok(0) => break,
                Ok(n) => {
                    fill += n;
                    if fill == block.len() {
                        if offset + fill > psize { overflow = true; break; }
                        unsafe { esp_partition_write(part, offset, block.as_ptr() as *const _, fill); }
                        offset += fill;
                        fill = 0;
                    }
                }
                Err(_) => break,
            }
        }
        if !overflow && fill > 0 {
            while fill % 4 != 0 { block[fill] = 0xFF; fill += 1; }
            if offset + fill <= psize {
                unsafe { esp_partition_write(part, offset, block.as_ptr() as *const _, fill); }
                offset += fill;
            } else {
                overflow = true;
            }
        }

        if overflow {
            info!("OTA-FS: image exceeds SPIFFS partition (>{} bytes)", psize);
            let mut resp = req.into_response(400, None, &[("Content-Type", "application/json")])?;
            resp.write_all(b"{\"error\":\"Image exceeds SPIFFS partition\"}")?;
            return Ok(());
        }

        info!("OTA-FS: SPIFFS written ({} bytes), reboot to mount", offset);
        send_ok(req)
    })?;

    // --- GET /api/display (framebuffer for web OLED mirror) ---
    let s = state.clone();
    server.fn_handler("/api/display", Method::Get, move |req| -> HandlerResult {
        if !is_authenticated(&req) { return send_unauthorized(req); }
        let st = s.lock().unwrap();
        let b64 = base64::engine::general_purpose::STANDARD.encode(&st.display_framebuffer);
        drop(st);
        let body = format!("{{\"width\":128,\"height\":64,\"data\":\"{}\"}}", b64);
        send_json_body(req, &body)
    })?;

    // --- GET /api/history (temperature history + extreme-heat change markers) ---
    let s = state.clone();
    server.fn_handler("/api/history", Method::Get, move |req| -> HandlerResult {
        if !is_authenticated(&req) { return send_unauthorized(req); }
        let st = s.lock().unwrap();
        let now_epoch = unsafe { esp_idf_svc::sys::time(std::ptr::null_mut()) } as i64;

        // Build the change-event marker array as a string. The (large) supply /
        // exhaust arrays are serialized by TempHistory::full_json directly to a
        // String — at 1440 buckets a serde value tree would be too memory-heavy.
        let mut events = String::from("[");
        for (i, e) in st.eh_events.iter().enumerate() {
            if i > 0 { events.push(','); }
            events.push_str(&format!(
                "{{\"epoch\":{},\"level\":{},\"reason\":\"{}\"}}",
                e.epoch, e.level, e.reason.as_str()
            ));
        }
        events.push(']');

        let body = st.temp_history.full_json(now_epoch, &events);
        drop(st);
        send_json_body(req, &body)
    })?;

    // --- GET /api/history/backup (compact restore-format snapshot, downloadable) ---
    // Same payload the device persists to MQTT, but fetched over HTTP so it can be
    // saved off-device and re-applied via /api/history/restore — independent of the
    // broker.
    let s = state.clone();
    server.fn_handler("/api/history/backup", Method::Get, move |req| -> HandlerResult {
        if !is_authenticated(&req) { return send_unauthorized(req); }
        let st = s.lock().unwrap();
        let now_epoch = unsafe { esp_idf_svc::sys::time(std::ptr::null_mut()) } as i64;
        let body = st.temp_history.snapshot_json(now_epoch);
        drop(st);
        send_json_body(req, &body)
    })?;

    // --- POST /api/history/restore (load a snapshot from /api/history/backup) ---
    let s = state.clone();
    server.fn_handler("/api/history/restore", Method::Post, move |mut req| -> HandlerResult {
        if !is_authenticated(&req) { return send_unauthorized(req); }
        let body = read_body(&mut req);
        let now_epoch = unsafe { esp_idf_svc::sys::time(std::ptr::null_mut()) } as i64;
        // Points are re-binned relative to "now", so the clock must be valid.
        if now_epoch < 1_700_000_000 {
            let mut resp = req.into_response(503, None, &[("Content-Type", "application/json")])?;
            resp.write_all(b"{\"error\":\"clock not synced yet\"}")?;
            return Ok(());
        }
        let ok = {
            let mut st = s.lock().unwrap();
            let applied = st.temp_history.restore_from_snapshot(&body, now_epoch);
            // Push the restored history straight back to the broker as the new
            // retained copy so it survives the next reboot.
            if applied { st.mqtt_publish_history = true; }
            applied
        };
        if ok {
            return send_ok(req);
        }
        let mut resp = req.into_response(400, None, &[("Content-Type", "application/json")])?;
        resp.write_all(b"{\"error\":\"could not parse snapshot\"}")?;
        Ok(())
    })?;

    // --- Static file serving (web UI from SPIFFS) ---
    // Serve specific known files
    for file in &["/app.js", "/style.css", "/favicon.ico"] {
        let path = format!("/www{}", file);
        server.fn_handler(file, Method::Get, move |req| -> HandlerResult {
            serve_file(req, &path)
        })?;
    }

    // Root path — serve index.html
    server.fn_handler("/", Method::Get, |req| -> HandlerResult {
        serve_file(req, "/www/index.html")
    })?;

    // SPA fallback — serve index.html for all other GET requests
    server.fn_handler("/*", Method::Get, |req| -> HandlerResult {
        serve_file(req, "/www/index.html")
    })?;

    info!("WebServer: Started on port 80");
    Ok(server)
}
