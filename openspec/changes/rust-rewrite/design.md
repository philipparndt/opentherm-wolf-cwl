## Context

The firmware has ~25 source files totaling ~5,500 lines. Key external dependencies: OpenTherm Library (C++, timing-critical GPIO), U8g2 (OLED display), PubSubClient (MQTT), ESPAsyncWebServer (HTTP), and ESP-IDF platform APIs (WiFi, Ethernet, NVS, I2C, WDT, OTA).

The current architecture uses global mutable state (`cwlData`, `appConfig`, `requestedVentLevel`, etc.) shared across modules. Rust requires explicit ownership — this shapes the entire design.

## Goals / Non-Goals

**Goals:**
- Feature-parity with the C++ firmware (all 6 display pages, MQTT pub/sub, web API, schedules, OTA, watchdog).
- FFI bridge to the existing OpenTherm C++ library — no reimplementation of Manchester encoding.
- Testable business logic (scheduler, config, data model) with unit tests.
- Same web UI (Preact SPA) served from the embedded filesystem.
- Build variants via Cargo features: `wifi`, `ethernet`, `simulate-ot`.

**Non-Goals:**
- async/await or RTOS task model — use a simple `loop()` like the C++ version for now.
- Removing the C++ version — both coexist.
- Changing any user-facing behavior or API contracts.

## Decisions

**Use `esp-idf-svc` (not bare-metal `no_std`).**

The firmware needs WiFi, Ethernet, NVS, HTTP server, mDNS, OTA, NTP — all provided by ESP-IDF. Using `esp-idf-svc` (std-based) gives access to the full ESP-IDF API with Rust ergonomics. This matches the Arduino framework's level of abstraction. The alternative (`embassy` / bare-metal) would require reimplementing networking stacks.

**FFI bridge for OpenTherm library via `cc` crate build script.**

The `ihormelnyk/OpenTherm` library is compiled as part of the Rust build using `cc::Build`. A thin `opentherm_ffi.h` / `opentherm_ffi.cpp` wrapper exposes a C-linkage API: `ot_init()`, `ot_send_request()`, `ot_get_response_status()`, `ot_process()`. Rust calls these via `extern "C"` bindings. This avoids reimplementing the timing-critical Manchester encoding/decoding.

**Shared application state via `AppState` struct passed as `&mut`.**

Replace globals with a single `AppState` struct containing `CwlData`, `AppConfig`, `SchedulerState`, and request channels. The main loop owns it and passes `&mut` references to subsystem update functions. No `Arc<Mutex<>>` needed since everything runs on one thread (main loop). ISR communication uses `AtomicI32` for encoder position (same pattern as C++ volatile).

**`embedded-graphics` with `ssd1306` crate for display.**

Replaces U8g2. The `ssd1306` crate supports both SSD1306 and SH1106 via feature flags. Font rendering uses `embedded-graphics` built-in fonts (similar sizes to U8g2's Helvetica). Layout logic is reimplemented but structurally identical (6 pages, edit mode, overlays, standby).

**`esp-idf-svc::mqtt` for MQTT client.**

ESP-IDF includes a built-in MQTT client (`esp_mqtt`). The `esp-idf-svc` crate wraps it with a safe Rust API. Replaces PubSubClient with equivalent pub/sub, LWT, and auto-reconnect.

**`esp-idf-svc::http::server` for web server.**

ESP-IDF's HTTP server replaces ESPAsyncWebServer. It's synchronous but handles concurrent connections via ESP-IDF's internal thread pool. Serves static files from SPIFFS/LittleFS. JSON serialization via `serde` + `serde_json` (replaces ArduinoJson).

**`esp-idf-svc::nvs` for configuration persistence.**

Direct replacement for Arduino `Preferences`. Same NVS namespace ("wolfcwl"), same key names for migration compatibility — a Rust-flashed device can read config written by the C++ firmware and vice versa.

**Phased implementation order.**

1. Project scaffold + OpenTherm FFI (proves the build toolchain works)
2. Data model + config manager (NVS, testable without hardware)
3. Network (WiFi/Ethernet) + web server (can test via browser)
4. MQTT client (can test with broker)
5. OpenTherm polling loop (integrates FFI, produces real data)
6. Display + encoder (I2C hardware)
7. Scheduler (pure logic, well-testable)
8. Watchdog, status LED, AP mode, OTA
9. Simulation mode feature flag

## Risks / Trade-offs

- **Display font differences**: `embedded-graphics` fonts don't match U8g2 Helvetica pixel-for-pixel. The layout will look slightly different but convey the same information. Acceptable since this is a development display, not a consumer product.
- **HTTP server is synchronous**: ESP-IDF's HTTP server uses a thread pool, not async. For this use case (one client at a time, small payloads) this is fine. OTA uploads work via chunked handlers, same as the async version.
- **NVS key compatibility**: Using the same namespace and key names means flashing Rust firmware on a device previously running C++ will pick up the existing config. However, if the Rust version adds new keys or changes types, a factory reset may be needed.
- **OpenTherm FFI complexity**: The C++ library uses interrupts (ISR) for timing. The FFI wrapper must ensure ISR registration works correctly from Rust. This is proven by `esp-idf-svc`'s GPIO interrupt support but needs careful testing.
