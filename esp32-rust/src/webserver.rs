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

fn serve_file(req: esp_idf_svc::http::server::Request<&mut EspHttpConnection>, path: &str) -> HandlerResult {
    match std::fs::read(path) {
        Ok(data) => {
            let ct = content_type(path);
            let mut resp = req.into_response(200, None, &[
                ("Content-Type", ct),
                ("Cache-Control", "public, max-age=3600"),
            ])?;
            resp.write_all(&data)?;
            Ok(())
        }
        Err(_) => {
            // SPA fallback: try index.html
            if path != "/www/index.html" {
                if let Ok(data) = std::fs::read("/www/index.html") {
                    let mut resp = req.into_response(200, None, &[("Content-Type", "text/html")])?;
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
                "supplyInlet": d.supply_inlet_temp,
                "exhaustInlet": d.exhaust_inlet_temp,
            },
            "status": {
                "fault": d.fault,
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
            },
            "timedOff": {
                "active": st.timed_off_active,
                "remainingMinutes": st.timed_off_remaining_min,
            },
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
            "configured": c.configured,
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
            if let Some(v) = val["configured"].as_bool() { st.config.configured = v; }
            info!("Config updated via POST /api/config");
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
            s.lock().unwrap().schedules = entries;
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
            s.lock().unwrap().bypass_schedule = schedule;
            return send_ok(req);
        }
        let mut resp = req.into_response(400, None, &[("Content-Type", "application/json")])?;
        resp.write_all(b"{\"error\":\"Invalid bypass schedule\"}")?;
        Ok(())
    })?;

    // --- POST /api/ota/upload (firmware OTA) ---
    server.fn_handler("/api/ota/upload", Method::Post, move |mut req| -> HandlerResult {
        if !is_authenticated(&req) { return send_unauthorized(req); }
        use esp_idf_svc::sys::*;

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
