mod app_state;
mod config;
mod config_manager;
mod cwl_data;
mod display;
mod encoder;
mod extreme_heat;
mod framebuffer;
mod history;
mod humidity;
pub mod i18n;
mod mqtt;
mod network;
mod opentherm;
mod ot_master;
mod panic_capture;
mod psychro;
mod scheduler;
mod status_led;
mod watchdog;
mod webserver;

use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::i2c::{I2cConfig, I2cDriver};
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::prelude::*;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use log::info;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Consume any pending encoder events, applying each to the display and
/// rendering between events so a fast spin shows every page. Called both
/// from the body of the main loop and from inside the sleep window so
/// rotation feels responsive without changing the main-loop cadence for
/// the rest of the work.
fn drain_encoder(
    enc: &mut Option<encoder::Encoder>,
    disp: &mut display::Display,
    now_ms: u32,
) {
    let Some(enc) = enc.as_mut() else { return };
    let start_us = unsafe { esp_idf_svc::sys::esp_timer_get_time() };
    let mut count = 0u32;
    while let Some(event) = enc.poll(now_ms) {
        match event {
            encoder::EncoderEvent::Rotate(delta) => {
                if !disp.wake() {
                    if disp.edit_mode {
                        disp.adjust_edit_value(delta);
                    } else if delta > 0 {
                        disp.next_page();
                    } else {
                        disp.prev_page();
                    }
                }
            }
            encoder::EncoderEvent::Press => {
                if !disp.wake() {
                    if disp.edit_mode {
                        disp.exit_edit_mode(true); // apply
                    } else {
                        disp.enter_edit_mode();
                    }
                }
            }
        }
        disp.update(now_ms);
        count += 1;
    }
    if count > 0 {
        let total_us = unsafe { esp_idf_svc::sys::esp_timer_get_time() } - start_us;
        info!("DRAIN events={} total={}us", count, total_us);
    }
}

fn main() {
    esp_idf_svc::log::EspLogger::initialize_default();

    // Capture future panics (message + file:line) across the reboot they cause,
    // then surface any panic from the previous run.
    panic_capture::install_hook();
    if let Some(p) = panic_capture::load() {
        log::error!("Recovered from previous panic: {p}");
    }

    info!("Wolf CWL - Rust Firmware");
    info!("Initializing...");

    let peripherals = Peripherals::take().unwrap();
    let sysloop = EspSystemEventLoop::take().unwrap();
    let nvs_partition = EspDefaultNvsPartition::take().unwrap();

    // Status LED
    let mut led = status_led::StatusLed::new(peripherals.pins.gpio2.into());

    // Wait for peripherals to stabilize after power-on (SH1106 needs ~100ms)
    FreeRtos::delay_ms(200);

    // I2C for display
    // 1 MHz brings the OLED flush down from ~32 ms (400 kHz, spec default) to
    // ~13 ms — the dominant cost of disp.update(). SSD1306 and SH1106 panels
    // routinely tolerate this on short, well-pulled-up buses; if the panel
    // ever shimmers or shows corruption, back this off to 400/800 kHz.
    let i2c_config = I2cConfig::new().baudrate(1.MHz().into());
    let i2c = I2cDriver::new(
        peripherals.i2c0,
        peripherals.pins.gpio13, // SDA
        peripherals.pins.gpio16, // SCL
        &i2c_config,
    ).unwrap();

    // Load config from NVS
    let config_mgr = config_manager::ConfigManager::new(nvs_partition.clone()).unwrap();
    let config = config_mgr.load();
    info!("Config loaded (configured={})", config.configured);

    // OT GPIO selection:
    //   ot-uext: uses UEXT TXD(GPIO1)/RXD(GPIO3) — default, matches SB3+SB4 closed
    //   ot-ext:  uses GPIO4/GPIO36 via J4 — fallback, matches SB1+SB2 closed
    #[cfg(feature = "ot-uext")]
    let (ot_in, ot_out) = {
        info!("OpenTherm: Using UEXT pins (GPIO3 RX, GPIO1 TX)");
        (3i32, 1i32)  // RXD=GPIO3 (OT input), TXD=GPIO1 (OT output)
    };
    #[cfg(feature = "ot-ext")]
    let (ot_in, ot_out) = {
        info!("OpenTherm: Using EXT pins (GPIO36 RX, GPIO4 TX)");
        (config.ot_in_pin as i32, config.ot_out_pin as i32)
    };
    #[cfg(not(any(feature = "ot-uext", feature = "ot-ext")))]
    let (ot_in, ot_out) = {
        info!("OpenTherm: Using config pins (GPIO{} RX, GPIO{} TX)", config.ot_in_pin, config.ot_out_pin);
        (config.ot_in_pin as i32, config.ot_out_pin as i32)
    };

    // Shared application state
    let state = app_state::new_app_state(config.clone());

    // Load schedules and timed-off state into shared state
    let restored_timed_off_end = config_mgr.load_timed_off_end();
    {
        let mut st = state.lock().unwrap();
        st.schedules = config_mgr.load_schedules();
        st.bypass_schedule = config_mgr.load_bypass_schedule();
        st.schedule_override = config_mgr.load_override_state();
        info!("Schedules: {} ventilation, bypass {}", st.schedules.len(),
              if st.bypass_schedule.enabled { "enabled" } else { "disabled" });
        if restored_timed_off_end > 0 {
            st.timed_off_end_epoch = restored_timed_off_end;
            info!("Timed off: restored end epoch {}", restored_timed_off_end);
        }
    }

    // Display + boot screen
    // Shared dirty flag — anything that mutates display-visible state (the
    // Display struct's own setters, plus background threads that touch
    // AppState) bumps it; Display::update() short-circuits when it's clear.
    let display_dirty: display::DisplayDirty = Arc::new(AtomicBool::new(true));
    let mut disp = display::Display::new(i2c, state.clone(), display_dirty.clone());
    disp.boot_screen();

    // Network setup — non-blocking. The driver starts in the background; the
    // main loop polls check_network_connected() and reacts (display, NTP)
    // when (and if) the link comes up. Without this, an unplugged cable
    // would block boot until wait_netif_up() times out.
    #[cfg(feature = "ethernet")]
    {
        match network::eth_impl::start_ethernet(
            peripherals.mac,
            peripherals.pins.gpio0,
            peripherals.pins.gpio12,
            peripherals.pins.gpio17,
            peripherals.pins.gpio18,
            peripherals.pins.gpio19,
            peripherals.pins.gpio21,
            peripherals.pins.gpio22,
            peripherals.pins.gpio23,
            peripherals.pins.gpio25,
            peripherals.pins.gpio26,
            peripherals.pins.gpio27,
            sysloop.clone(),
        ) {
            Ok(_) => info!("Network: ETH driver started (no link yet)"),
            Err(e) => info!("Network: ETH init failed ({}), continuing offline", e),
        }
    }

    #[cfg(all(feature = "wifi", not(feature = "ethernet")))]
    {
        use network::wifi_impl::WifiNetwork;
        // Fall back to credentials baked in at build time (from esp32/.env via
        // build.rs) when NVS is empty — lets a freshly-flashed device join the
        // network without going through the web setup first.
        const BUILD_WIFI_SSID: &str = env!("WIFI_SSID");
        const BUILD_WIFI_PASSWORD: &str = env!("WIFI_PASSWORD");
        let ssid = if !config.wifi_ssid.is_empty() { config.wifi_ssid.as_str() } else { BUILD_WIFI_SSID };
        let password = if !config.wifi_password.is_empty() { config.wifi_password.as_str() } else { BUILD_WIFI_PASSWORD };
        match WifiNetwork::new(peripherals.modem, sysloop.clone(), nvs_partition.clone()) {
            Ok(mut wifi) => {
                wifi.connect(ssid, password).ok();
                let connected = wifi.state.connected;
                if connected {
                    let mut st = state.lock().unwrap();
                    st.network_connected = connected;
                    st.ip_address = wifi.state.ip_address.clone();
                }
                info!("Network: WiFi connected={}, ip={}", connected,
                      wifi.state.ip_address.as_deref().unwrap_or("none"));
                std::mem::forget(wifi);
            }
            Err(e) => info!("Network: WiFi init failed ({}), continuing offline", e),
        }
    }

    // Mount SPIFFS filesystem for web UI
    {
        use esp_idf_svc::sys::*;
        let conf = esp_vfs_spiffs_conf_t {
            base_path: b"/www\0".as_ptr() as *const _,
            partition_label: b"spiffs\0".as_ptr() as *const _,
            max_files: 5,
            format_if_mount_failed: false,
        };
        let ret = unsafe { esp_vfs_spiffs_register(&conf) };
        if ret == ESP_OK {
            info!("SPIFFS: Mounted at /www");
        } else {
            info!("SPIFFS: Mount failed (err={}) — web UI not available", ret);
        }
    }

    // Encoder
    let mut enc = encoder::Encoder::new(
        peripherals.pins.gpio15.into(),
        peripherals.pins.gpio14.into(),
        peripherals.pins.gpio5.into(),
    ).ok();

    // MQTT also runs on its own thread — publish bursts (~25 topics over the
    // network) can take 50–200 ms and we don't want that anywhere near the
    // encoder/display path. Incoming-command callbacks fire on EspMqttClient's
    // internal event-loop thread, independently of this one.
    {
        let state = state.clone();
        std::thread::Builder::new()
            .name("mqtt".into())
            .stack_size(8192)
            .spawn(move || {
                let mut mqtt_mgr = match mqtt::MqttManager::new(state) {
                    Some(m) => m,
                    None => return, // disabled / not configured
                };
                mqtt_mgr.setup_subscriptions();
                loop {
                    let now_ms = unsafe { (esp_idf_svc::sys::esp_timer_get_time() / 1000) as u32 };
                    mqtt_mgr.update(now_ms);
                    FreeRtos::delay_ms(100);
                }
            })
            .expect("Failed to spawn MQTT thread");
    }

    // OpenTherm master
    // OpenTherm runs on its own thread so its blocking I/O (up to ~1 s per
    // request with a real, unresponsive slave) doesn't stall the main loop's
    // UI / encoder / display work.
    {
        let state = state.clone();
        let display_dirty = display_dirty.clone();
        std::thread::Builder::new()
            .name("ot".into())
            .stack_size(16384)
            .spawn(move || {
                let mut ot = ot_master::OtMaster::new(state, ot_in, ot_out)
                    .expect("Failed to init OpenTherm");
                loop {
                    let now_ms = unsafe { (esp_idf_svc::sys::esp_timer_get_time() / 1000) as u32 };
                    // update() returns true when it actually ran a poll step
                    // (~1 Hz). Only then might cwl_data have changed, so we
                    // only flip the dirty flag at that cadence.
                    if ot.update(now_ms) {
                        display_dirty.store(true, Ordering::Relaxed);
                    }
                    // ot.update() throttles itself to POLL_INTERVAL_MS (1 s).
                    // Sleep short enough that timed_off_request / vent level
                    // changes get serviced on the next tick.
                    FreeRtos::delay_ms(100);
                }
            })
            .expect("Failed to spawn OT thread");
    }

    // Scheduler
    let mut sched = scheduler::Scheduler::new(state.clone());
    {
        let st = state.lock().unwrap();
        sched.schedules = st.schedules.clone();
        sched.bypass_schedule = st.bypass_schedule.clone();
        // Restore timed-off from NVS (deferred NTP validation happens in scheduler.update())
        if restored_timed_off_end > 0 {
            sched.timed_off_active = true;
            sched.timed_off_end_epoch = restored_timed_off_end;
            // Set ventilation to off until NTP validates
            // (scheduler.update() will clear if expired)
        }
    }

    // Extreme-heat automatic mode — drives the ventilation level from the
    // supply-vs-exhaust temperature difference when enabled in config.
    let mut extreme_heat = extreme_heat::ExtremeHeat::new(state.clone());

    // Watchdog
    let mut wdt = watchdog::Watchdog::new();

    // Web server
    let _server = webserver::start_server(state.clone()).expect("Failed to start web server");

    info!("Setup complete, entering main loop");
    info!("Reboot reason: {}", watchdog::reboot_reason());

    let mut last_net_connected = state.lock().unwrap().network_connected;
    let mut last_net_check_ms: u32 = 0;
    const NET_CHECK_INTERVAL_MS: u32 = 2000;
    let mut ntp_initialized = false;

    // WiFi init blocks until connected, so by the time we enter the loop the
    // network is already up and the "disconnected → connected" edge that
    // normally triggers setup_ntp() never fires. Kick NTP off explicitly here
    // when we already have a link.
    if last_net_connected {
        ntp_initialized = true;
        network::setup_ntp();
        network::setup_mdns().ok();
    }

    // LED RX activity blink: invert the LED briefly when fresh slave activity
    // is observed (Success or Invalid framing — both bumped by ot_master).
    // Pulse spans several iterations of the 5 ms inner idle loop so a response
    // landing mid-window is still visible.
    let mut last_seen_rx_counter: u32 = 0;
    let mut rx_pulse_start_ms: u32 = 0;
    const RX_PULSE_MS: u32 = 300;

    // One-shot: record a "reboot" history marker once the clock is valid and any
    // retained markers have been restored (so it appends rather than being wiped).
    let mut reboot_marked = false;

    loop {
        let now_ms = unsafe { (esp_idf_svc::sys::esp_timer_get_time() / 1000) as u32 };

        // OpenTherm polling now runs on its own thread (spawned above).

        // Timed off requests from web/MQTT
        {
            let mut st = state.lock().unwrap();
            if let Some(minutes) = st.timed_off_request.take() {
                drop(st);
                sched.activate_timed_off(minutes);
            } else if st.cancel_timed_off {
                st.cancel_timed_off = false;
                drop(st);
                sched.cancel_timed_off();
            } else {
                drop(st);
            }
        }

        // Scheduler
        sched.update(now_ms);

        // Extreme-heat automatic mode (no-op unless enabled in config). Runs
        // after the scheduler, which suppresses its own ventilation level
        // changes while the mode owns requested_vent_level.
        extreme_heat.update(now_ms);

        // Persist timed-off state to NVS when changed
        {
            let mut st = state.lock().unwrap();
            if st.persist_timed_off {
                st.persist_timed_off = false;
                let end_epoch = st.timed_off_end_epoch;
                let override_active = st.schedule_override;
                drop(st);
                let mut mgr = config_manager::ConfigManager::new(nvs_partition.clone()).unwrap();
                mgr.save_timed_off_end(end_epoch).ok();
                mgr.save_override_state(override_active).ok();
            }
        }

        // Persist full config to NVS when requested (e.g. language change)
        {
            let mut st = state.lock().unwrap();
            if st.persist_config {
                st.persist_config = false;
                let config = st.config.clone();
                drop(st);
                let mut mgr = config_manager::ConfigManager::new(nvs_partition.clone()).unwrap();
                mgr.save(&config).ok();
            }
        }

        // Persist ventilation + bypass schedules to NVS when changed via the web UI
        {
            let mut st = state.lock().unwrap();
            if st.persist_schedules {
                st.persist_schedules = false;
                let schedules = st.schedules.clone();
                let bypass_schedule = st.bypass_schedule.clone();
                drop(st);
                let mut mgr = config_manager::ConfigManager::new(nvs_partition.clone()).unwrap();
                mgr.save_schedules(&schedules).ok();
                mgr.save_bypass_schedule(&bypass_schedule).ok();
            }
        }

        // MQTT publishing now runs on its own thread (spawned above).

        // Watchdog (LED state itself is driven from the 5 ms inner loop below
        // so RX pulses arriving mid-iteration are not missed).
        {
            let st = state.lock().unwrap();
            wdt.update(now_ms, st.cwl_data.last_response_ms, st.cwl_data.connected);
        }

        // Temperature history sampler — fold latest readings into 24 h
        // ring buffers. Mark the display dirty only when a bucket rolls
        // over (~once every 11 min) so we don't re-render the chart every
        // second.
        {
            let mut st = state.lock().unwrap();
            let st = &mut *st;
            let rolled = st.temp_history.sample(now_ms, &st.cwl_data);
            if rolled {
                display_dirty.store(true, Ordering::Relaxed);
                // Persist the rolled-over history to the broker (retained).
                st.mqtt_publish_history = true;
            }
            // Coarse humidity history: max fresh indoor RH + fresh outdoor RH.
            let indoor_rh = st.humidity_inside.values()
                .filter(|s| humidity::is_fresh(s.updated_ms, now_ms))
                .map(|s| s.humidity)
                .fold(None, |acc: Option<f32>, h| Some(acc.map_or(h, |a| a.max(h))));
            let outdoor_rh = st.humidity_outside
                .filter(|s| humidity::is_fresh(s.updated_ms, now_ms))
                .map(|s| s.humidity);
            st.temp_history.sample_humidity(now_ms, indoor_rh, outdoor_rh);
        }

        // Apply any RAM-only state recovered from retained MQTT snapshots. The
        // MQTT callback only stashes raw bytes; the heavier JSON parse happens
        // here on the main loop.
        //
        // Crucially, wait until the wall clock is valid (NTP synced): the history
        // snapshot re-bins each point by its epoch relative to "now", so restoring
        // against a 1970 clock maps every point out of range and silently drops
        // the whole history. MQTT often delivers the retained snapshot before NTP
        // finishes (notably on Ethernet), so we leave the bytes *pending* — not
        // taken — until the clock is good, then restore on a later iteration.
        {
            let now_epoch = unsafe { esp_idf_svc::sys::time(std::ptr::null_mut()) } as i64;
            if now_epoch >= 1_700_000_000 {
                let mut st = state.lock().unwrap();
                if let Some(bytes) = st.pending_history_json.take() {
                    let st = &mut *st;
                    if st.temp_history.restore_from_snapshot(&bytes, now_epoch) {
                        display_dirty.store(true, Ordering::Relaxed);
                        info!("MQTT recovery: temperature history restored");
                    }
                }
                if let Some(bytes) = st.pending_extreme_heat_json.take() {
                    let st = &mut *st;
                    extreme_heat::restore_snapshot(st, &bytes);
                    info!("MQTT recovery: extreme-heat markers restored");
                }
                // Mark the reboot once, after any retained markers were restored
                // (restore replaces the event list, so this must come after it).
                if !reboot_marked {
                    reboot_marked = true;
                    let level = st.requested_vent_level;
                    st.push_history_marker_now(level, crate::app_state::Reason::Reboot);
                }
            }
        }

        // Virtual encoder from web UI
        {
            let action = state.lock().unwrap().encoder_action.take();
            if let Some(action) = action {
                match action.as_str() {
                    "left" => {
                        if !disp.wake() {
                            if disp.edit_mode { disp.adjust_edit_value(-1); }
                            else { disp.prev_page(); }
                        }
                    }
                    "right" => {
                        if !disp.wake() {
                            if disp.edit_mode { disp.adjust_edit_value(1); }
                            else { disp.next_page(); }
                        }
                    }
                    "press" => {
                        if !disp.wake() {
                            if disp.edit_mode { disp.exit_edit_mode(true); }
                            else { disp.enter_edit_mode(); }
                        }
                    }
                    _ => {}
                }
            }
        }

        // Check for display wake requests (from MQTT thread)
        {
            let mut st = state.lock().unwrap();
            if st.display_wake_requested {
                st.display_wake_requested = false;
                drop(st);
                disp.wake();
            }
        }

        // Network state monitoring
        if now_ms.wrapping_sub(last_net_check_ms) >= NET_CHECK_INTERVAL_MS {
            last_net_check_ms = now_ms;
            let (connected, ip) = network::check_network_connected();
            if connected != last_net_connected {
                last_net_connected = connected;
                {
                    let mut st = state.lock().unwrap();
                    st.network_connected = connected;
                    st.ip_address = ip.clone();
                }
                if connected {
                    if let Some(ref ip) = ip {
                        info!("Network: Connected, IP: {}", ip);
                        disp.show_ip(ip);
                    }
                    if !ntp_initialized {
                        ntp_initialized = true;
                        network::setup_ntp();
                        network::setup_mdns().ok();
                    }
                } else {
                    info!("Network: Disconnected");
                    disp.show_disconnected();
                }
            }
        }

        // Encoder input
        drain_encoder(&mut enc, &mut disp, now_ms);

        // Factory reset — encoder long-hold (10s)
        if let Some(ref enc) = enc {
            if enc.is_long_hold(now_ms) {
                info!("Factory reset triggered!");
                let mut mgr = config_manager::ConfigManager::new(nvs_partition.clone()).unwrap();
                mgr.reset().ok();
                unsafe { esp_idf_svc::sys::esp_restart() };
            }
        }

        // Display update
        disp.update(now_ms);

        // Idle window — broken into short slices so encoder events get
        // serviced within ~10 ms instead of waiting for the next full
        // iteration (~80 ms). The encoder poll itself is a single atomic
        // read, so this loop is effectively free.
        for _ in 0..10 {
            FreeRtos::delay_ms(5);
            let now_ms = unsafe { (esp_idf_svc::sys::esp_timer_get_time() / 1000) as u32 };
            drain_encoder(&mut enc, &mut disp, now_ms);

            // Drive LED here (every 5 ms) so a fresh RX is reflected in the
            // pulse window even if it lands between outer-loop iterations.
            let (rx_counter, net_ok) = {
                let st = state.lock().unwrap();
                (st.cwl_data.rx_seen_counter, st.network_connected && st.mqtt_connected)
            };
            if rx_counter != last_seen_rx_counter {
                last_seen_rx_counter = rx_counter;
                rx_pulse_start_ms = now_ms;
            }
            let in_pulse = now_ms.wrapping_sub(rx_pulse_start_ms) < RX_PULSE_MS
                && rx_pulse_start_ms != 0;
            led.set(net_ok ^ in_pulse);
        }
    }
}
