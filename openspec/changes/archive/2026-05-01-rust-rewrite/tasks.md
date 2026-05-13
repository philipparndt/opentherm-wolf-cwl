## 1. Project Scaffold

- [x] 1.1 Create `esp32-rust/` directory with Cargo workspace, `.cargo/config.toml` for ESP32 target, `rust-toolchain.toml` (esp channel)
- [x] 1.2 Set up `Cargo.toml` with dependencies: `esp-idf-svc`, `esp-idf-hal`, `embedded-graphics`, `ssd1306`, `serde`, `serde_json`, `log`
- [x] 1.3 Create minimal `src/main.rs` that boots, prints to serial, and blinks the status LED — verify `cargo build` and `espflash` work
- [x] 1.4 Add Makefile with targets: `build`, `flash`, `monitor`, `flash-all` (firmware + filesystem)

## 2. OpenTherm FFI Bridge

- [x] 2.1 Copy OpenTherm library source into `esp32-rust/components/opentherm/`
- [x] 2.2 Create `opentherm_ffi.h` / `opentherm_ffi.cpp` C-linkage wrapper: `ot_init(in_pin, out_pin)`, `ot_send_request(msg) -> response`, `ot_get_status() -> status`, `ot_process()` (ISR tick)
- [x] 2.3 Add `build.rs` using `cc` crate to compile the OpenTherm library + FFI wrapper
- [x] 2.4 Create `src/opentherm_ffi.rs` with `extern "C"` bindings and safe Rust wrapper types
- [x] 2.5 Arduino compatibility shim (Arduino.h → ESP-IDF GPIO/timer APIs) for OpenTherm library, cross-compiled with xtensa-esp32-elf-g++

## 3. Data Model

- [x] 3.1 Create `src/cwl_data.rs`: `CwlData` struct (temperatures, ventilation level, TSP registers, status flags, feature support flags)
- [x] 3.2 Create `src/config.rs`: `AppConfig` struct with serde derive, default values matching C++ defaults
- [x] 3.3 Add helper functions: `ventilation_level_name()`, `f8_8_to_float()`, TSP decoding
- [x] 3.4 Write unit tests for data model helpers

## 4. Config Manager (NVS)

- [x] 4.1 Create `src/config_manager.rs`: load/save `AppConfig` from NVS using `esp-idf-svc::nvs`, same namespace and key names as C++ version
- [x] 4.2 Implement `save_bypass_state()` (single-key write for bypass persistence)
- [x] 4.3 Implement `reset_config()` (clear all NVS keys)
- [ ] 4.4 Write unit tests for config serialization/deserialization

## 5. Network

- [x] 5.1 Create `src/network.rs` with WiFi support (feature-gated `wifi`): connect, auto-reconnect, status tracking
- [x] 5.2 Add Ethernet support (feature-gated `ethernet`): Olimex ESP32-POE event-driven setup
- [x] 5.3 Integrate display overlay calls (IP on connect, "Disconnected" on loss) — via main loop network monitor
- [x] 5.4 Set hostname "wolf-cwl" and configure NTP

## 6. Web Server

- [x] 6.1 Create `src/webserver.rs` using `esp-idf-svc::http::server`: cookie-based auth, static file serving from SPIFFS
- [x] 6.2 Implement status API: `GET /api/status`, `POST /api/login`
- [x] 6.3 Implement config API: `GET /api/config`, `POST /api/config`
- [x] 6.4 Implement ventilation API: `POST /api/ventilation/level`, `POST /api/ventilation/resume`
- [x] 6.5 Implement schedule API: `GET /api/schedules`, `POST /api/schedules`, `GET /api/bypass-schedule`, `POST /api/bypass-schedule`
- [x] 6.6 Implement utility API: `POST /api/encoder`, `POST /api/off_timer/cancel`
- [x] 6.7 Implement OTA: `POST /api/ota/upload`
- [ ] 6.8 Implement WiFi setup API for AP mode: `POST /api/wifi/setup`
- [x] 6.9 Add mDNS registration ("wolf-cwl.local") — placeholder, needs sdkconfig rebuild

## 7. MQTT Client

- [x] 7.1 Create `src/mqtt.rs` using `esp-idf-svc::mqtt::client`: connect with config, LWT, auto-reconnect
- [x] 7.2 Implement publish cycle: sensor data (11s), health (60s), bridge info (on connect)
- [x] 7.3 Implement subscribe + callback: set/level, set/bypass, set/filter_reset, set/off_timer
- [x] 7.4 Integrate display wake on MQTT commands

## 8. OpenTherm Polling Loop

- [x] 8.1 Create `src/ot_master.rs`: polling state machine (14 states, 1s interval) using FFI bridge
- [x] 8.2 Implement request builders: status (ID 70 with bypass flags), setpoint (ID 71), temperatures, TSP, fault
- [x] 8.3 Implement response processors: populate `CwlData` from responses
- [x] 8.4 Implement startup probe for optional data IDs (78-88)
- [x] 8.5 Add simulation mode (feature-gated `simulate-ot`): sinusoidal temperatures, fake TSP data

## 9. Display

- [x] 9.1 Create `src/display.rs`: I2C OLED init with auto-detect SSD1306/SH1106, standby management
- [x] 9.2 Implement page renderers: Home, Bypass, TempIn, TempOut, Status, System
- [x] 9.3 Implement edit mode: level selection, timeout, encoder integration
- [x] 9.4 Implement overlay system: network status (Connected/Disconnected), CWL disconnected screen
- [x] 9.5 Implement page indicator dots and SIM label (feature-gated)
- [x] 9.6 Implement `display_wake()`, `display_show_ip()`, `display_show_disconnected()`

## 10. Encoder

- [x] 10.1 Create `src/encoder.rs`: GPIO polling-based rotary encoder with debounce
- [x] 10.2 Implement rotation → page navigation / edit value adjustment
- [x] 10.3 Implement button press → edit mode toggle, standby wake

## 11. Scheduler

- [x] 11.1 Create `src/scheduler.rs`: schedule evaluation logic (time range, day bitmask, date range for bypass)
- [x] 11.2 Implement manual override: set, clear, persist to NVS
- [x] 11.3 Implement timed-off: activate, cancel, NTP-deferred validation on boot
- [x] 11.4 Implement NVS load/save for schedules and bypass schedule
- [x] 11.5 Implement JSON serialization for web API (serde derives on ScheduleEntry, BypassSchedule)
- [x] 11.6 Write unit tests for schedule evaluation, time range matching, date range matching

## 12. System Services

- [x] 12.1 Create `src/watchdog.rs`: task WDT (30s), liveness checks (OT 5min, heap 20KB)
- [x] 12.2 Create `src/status_led.rs`: status LED (GPIO)
- [ ] 12.3 Create `src/ap_mode.rs`: soft AP, DNS captive portal (feature-gated, WiFi only)
- [ ] 12.4 Create `src/logging.rs`: serial + WebSocket broadcast, ISO 8601 timestamps

## 13. Main Loop Integration

- [x] 13.1 Create `AppState` struct in `src/app_state.rs` with `Arc<Mutex<>>`
- [x] 13.2 Wire up main loop: OT polling → MQTT publish → display update → scheduler eval → watchdog feed
- [x] 13.3 Wire up factory reset (encoder long-hold 10s)
- [x] 13.4 Add boot sequence: config load → boot screen → network → SPIFFS → MQTT → OT → display → encoder → scheduler

## 14. Build & Deploy

- [x] 14.1 Set up partition table via espflash --partition-table flag
- [x] 14.2 Add web UI build integration (make build-web/build-fs/flash-fs/flash-all)
- [ ] 14.3 Verify full firmware flash + web UI on real hardware
