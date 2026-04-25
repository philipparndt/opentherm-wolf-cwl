## 1. Project Scaffolding

- [x] 1.1 Create `esp32/` directory with PlatformIO project (`platformio.ini`, `Makefile`, partition table)
- [x] 1.2 Create `include/config.h` with compile-time defaults (pins, MQTT topic, AP credentials)
- [x] 1.3 Create `src/logging.h` with serial + WebSocket logging macros

## 2. Configuration & Network

- [x] 2.1 Implement `config_manager.cpp/.h` — NVS load/save for WiFi, MQTT, pin assignments, ventilation state
- [x] 2.2 Implement `network.cpp/.h` — WiFi STA connection, reconnection logic, event handlers
- [x] 2.3 Implement `ap_mode.cpp/.h` — WiFi AP mode with captive portal for first-run setup
- [x] 2.4 Implement `status.cpp/.h` — LED status indication for connection states

## 3. OpenTherm Core

- [x] 3.1 Implement `wolf_cwl.cpp/.h` — data model struct for all sensor values, status flags, TSP cache, ventilation level, bypass state
- [x] 3.2 Implement `ot_master.cpp/.h` — polling cycle state machine (IDs 2, 70, 71, 72, 74, 77, 80, 82, 89, 126, 127), response parsing, TSP index rotation
- [x] 3.3 Add bypass control logic — set/clear bit 1 in ID 70 HI byte based on bypass request state
- [x] 3.4 Add filter reset logic — set bit 4 in ID 70 HI byte for one cycle when triggered
- [x] 3.5 Add startup probe for additional data IDs (78, 79, 81, 83, 84, 85, 87, 88)

## 4. MQTT Integration

- [x] 4.1 Implement `mqtt_client.cpp/.h` — connection, LWT, reconnection with 5s retry
- [x] 4.2 Add MQTT publishing — all sensor values after each polling cycle (temperatures, ventilation, status, TSP)
- [x] 4.3 Add MQTT subscriptions — `set/level`, `set/bypass`, `set/filter_reset` command handling

## 5. OLED Display

- [x] 5.1 Implement `display.cpp/.h` — U8g2 initialization, page rendering framework, boot screen
- [x] 5.2 Implement home page — ventilation level (large text), relative %, bypass state
- [x] 5.3 Implement temperature page — supply inlet and exhaust inlet temperatures
- [x] 5.4 Implement status page — fault, filter, bypass, airflow m³/h
- [x] 5.5 Implement system page — WiFi RSSI, MQTT state, IP, uptime

## 6. Rotary Encoder

- [x] 6.1 Implement `encoder.cpp/.h` — encoder initialization, rotation detection, button press with debounce
- [x] 6.2 Add page navigation — rotate to switch display pages
- [x] 6.3 Add edit mode — press to enter, rotate to adjust ventilation level, press to confirm, 10s timeout

## 7. Web Server & UI

- [x] 7.1 Implement `webserver.cpp/.h` — ESPAsyncWebServer, JWT auth, mDNS, LittleFS mount
- [x] 7.2 Add API endpoints — `/api/config` (get/set), `/api/status` (current state), `/api/ota/upload`
- [x] 7.3 Create Preact web UI — login page, settings tab (WiFi, MQTT, pins), status tab, system tab with OTA

## 8. Main Loop & Integration

- [x] 8.1 Implement `main.cpp` — setup() with initialization order, loop() orchestrating all components
- [x] 8.2 Add factory reset — hold encoder button 10s during boot to clear NVS
- [x] 8.3 Integration testing — firmware compiles successfully (16% RAM, 60% flash), web UI builds (21KB JS + 2KB CSS)
