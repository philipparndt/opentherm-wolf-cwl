mod app_state;
mod config;
mod config_manager;
mod cwl_data;
mod display;
mod encoder;
mod framebuffer;
pub mod i18n;
mod mqtt;
mod network;
#[cfg(not(feature = "simulate-ot"))]
mod opentherm_ffi;
mod ot_master;
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

fn main() {
    esp_idf_svc::log::EspLogger::initialize_default();

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
    let i2c_config = I2cConfig::new().baudrate(400.kHz().into());
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
    let mut disp = display::Display::new(i2c, state.clone());
    disp.boot_screen();

    // Network setup
    #[cfg(feature = "ethernet")]
    {
        let net = network::eth_impl::start_ethernet(
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
        ).unwrap();

        {
            let mut st = state.lock().unwrap();
            st.network_connected = net.connected;
            st.ip_address = net.ip_address.clone();
        }
        if net.connected {
            if let Some(ref ip) = net.ip_address {
                disp.show_ip(ip);
            }
            network::setup_ntp();
            network::setup_mdns().ok();
        }
        info!("Network: connected={}, ip={}", net.connected,
              net.ip_address.as_deref().unwrap_or("none"));
    }

    #[cfg(all(feature = "wifi", not(feature = "ethernet")))]
    {
        use network::wifi_impl::WifiNetwork;
        let mut wifi = WifiNetwork::new(peripherals.modem, sysloop.clone(), nvs_partition.clone()).unwrap();
        wifi.connect(&config.wifi_ssid, &config.wifi_password).ok();
        let connected = wifi.state.connected;
        {
            let mut st = state.lock().unwrap();
            st.network_connected = connected;
            st.ip_address = wifi.state.ip_address.clone();
        }
        if connected {
            if let Some(ref ip) = wifi.state.ip_address {
                disp.show_ip(ip);
            }
            network::setup_ntp();
            network::setup_mdns().ok();
        }
        info!("Network: connected={}, ip={}", connected,
              wifi.state.ip_address.as_deref().unwrap_or("none"));
        std::mem::forget(wifi);
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

    // MQTT
    let mut mqtt_mgr = mqtt::MqttManager::new(state.clone());
    if let Some(ref mut mgr) = mqtt_mgr {
        mgr.setup_subscriptions();
    }

    // OpenTherm master
    let mut ot = ot_master::OtMaster::new(state.clone(), ot_in, ot_out)
        .expect("Failed to init OpenTherm");

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

    // Watchdog
    let mut wdt = watchdog::Watchdog::new();

    // Web server
    let _server = webserver::start_server(state.clone()).expect("Failed to start web server");

    info!("Setup complete, entering main loop");
    info!("Reboot reason: {}", watchdog::reboot_reason());

    let mut last_net_connected = state.lock().unwrap().network_connected;
    let mut last_net_check_ms: u32 = 0;
    const NET_CHECK_INTERVAL_MS: u32 = 2000;

    loop {
        let now_ms = unsafe { (esp_idf_svc::sys::esp_timer_get_time() / 1000) as u32 };

        // OpenTherm polling
        ot.update(now_ms);

        // Timed off requests from web/MQTT
        {
            let mut st = state.lock().unwrap();
            if let Some(hours) = st.timed_off_request.take() {
                drop(st);
                sched.activate_timed_off(hours);
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

        // MQTT publishing
        if let Some(ref mut mgr) = mqtt_mgr {
            mgr.update(now_ms);
        }

        // Watchdog
        {
            let st = state.lock().unwrap();
            wdt.update(now_ms, st.cwl_data.last_response_ms, st.cwl_data.connected);
            // Status LED: on when network + MQTT connected
            led.set(st.network_connected && st.mqtt_connected);
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
                } else {
                    info!("Network: Disconnected");
                    disp.show_disconnected();
                }
            }
        }

        // Encoder input
        if let Some(ref mut enc) = enc {
            if let Some(event) = enc.poll(now_ms) {
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
            }
        }

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

        FreeRtos::delay_ms(50);
    }
}
