## Context

The Wolf CWL 300 is a heat-recovery ventilation unit controlled via OpenTherm protocol. The existing BM remote polls the unit in ~11-second cycles using V/H data IDs (70-91). A Go-based protocol decoder in this repository has fully reverse-engineered the communication, documenting all supported data IDs, control mechanisms, and TSP register maps.

The DIYLess Master OpenTherm Shield handles the electrical interface (voltage modulation/current detection) and exposes two GPIO pins (input/output) to the ESP32. The ihormelnyk OpenTherm library provides the software layer for Manchester encoding/decoding and frame construction.

The reference project (unifi-access/esp32) provides a proven architecture for ESP32 IoT devices with WiFi, MQTT, web UI, and OTA updates.

## Goals / Non-Goals

**Goals:**
- Full replacement of the BM remote control via ESP32 + DIYLess shield
- Read all sensor data: temperatures, ventilation %, status flags, fault codes, TSP registers
- Control ventilation level (0-3) and bypass damper via MQTT commands
- Local OLED display showing current state with rotary encoder for control
- Web-based configuration for WiFi, MQTT, and pin assignments
- OTA firmware updates
- Probe additional V/H data IDs not used by the BM remote (IDs 78, 79, 81, 83-85, 87, 88)

**Non-Goals:**
- Simultaneous operation with BM remote (OpenTherm is single-master)
- ESPHome integration (ESPHome lacks V/H support; this is standalone firmware)
- Ethernet support (WiFi only for this board)
- Web UI for real-time data visualization (MQTT + Home Assistant handles this)

## Decisions

### 1. Project structure mirrors reference project

```
esp32/
├── src/
│   ├── main.cpp              # Setup + main loop
│   ├── opentherm.cpp/.h      # OpenTherm polling cycle, data ID handling
│   ├── wolf_cwl.cpp/.h       # Wolf CWL data model, TSP register map
│   ├── mqtt_client.cpp/.h    # MQTT publish/subscribe
│   ├── display.cpp/.h        # OLED display pages
│   ├── encoder.cpp/.h        # Rotary encoder input
│   ├── network.cpp/.h        # WiFi connection management
│   ├── config_manager.cpp/.h # NVS configuration
│   ├── webserver.cpp/.h      # Async web server + API
│   ├── ap_mode.cpp/.h        # WiFi AP for initial setup
│   ├── status.cpp/.h         # LED status indication
│   └── logging.h             # Serial + WebSocket logging
├── web/                      # Preact web UI
├── include/
│   └── config.h              # Compile-time defaults
├── platformio.ini
└── Makefile
```

**Rationale:** Follows the proven pattern from the reference project. Each concern gets its own compilation unit with a header for cross-file access.

### 2. OpenTherm library: ihormelnyk/opentherm_library

**Rationale:** This is the de facto Arduino OpenTherm library, specifically designed for the DIYLess shields. It handles Manchester encoding, frame timing, and interrupt-driven communication. No need to reimplement.

**Alternative considered:** Raw GPIO with custom Manchester encoding — rejected as unnecessary complexity.

### 3. Polling architecture: sequential in main loop with millis()-based timing

The main loop runs an OpenTherm polling cycle that sends one request per second (matching the BM remote's timing). Each cycle of ~11 requests completes in ~11 seconds. TSP register index rotates each cycle.

**Rationale:** OpenTherm is inherently sequential (one request-response at a time). A simple state machine stepping through the request list with 1-second intervals is clean and matches the protocol.

**Alternative considered:** FreeRTOS task — rejected because the OpenTherm library uses interrupts and is designed for single-task Arduino usage.

### 4. OLED: U8g2 library with SSD1306 128x64 I2C

**Rationale:** U8g2 is the most comprehensive monochrome display library, supports hardware I2C, has excellent font selection, and handles buffered rendering. It supports the SSD1306 and SH1110 variants.

**Alternative considered:** Adafruit SSD1306 — simpler but fewer fonts and less flexible layout control.

### 5. Display pages with rotary encoder navigation

Pages:
1. **Home**: Ventilation level (large), relative ventilation %, bypass state
2. **Temperatures**: Supply inlet, exhaust inlet (and outlet if supported)
3. **Status**: Fault flags, filter status, bypass status, current airflow m³/h
4. **System**: WiFi RSSI, MQTT state, IP address, uptime

Encoder: rotate to switch pages, press to enter edit mode (e.g., change ventilation level), rotate to adjust, press to confirm.

### 6. MQTT topic structure

```
wolf-cwl/ventilation/level          → 0-3
wolf-cwl/ventilation/relative       → 0-100 (%)
wolf-cwl/temperature/supply_inlet   → °C (f8.8)
wolf-cwl/temperature/exhaust_inlet  → °C (f8.8)
wolf-cwl/status/fault               → 0/1
wolf-cwl/status/filter              → 0/1
wolf-cwl/status/bypass              → 0/1
wolf-cwl/tsp/<index>                → raw value
wolf-cwl/bridge/state               → online/offline (LWT)
wolf-cwl/bridge/version             → firmware version
wolf-cwl/set/level                  → command: 0-3
wolf-cwl/set/bypass                 → command: 0/1
wolf-cwl/set/filter_reset           → command: 1 (trigger)
```

**Rationale:** Flat topic structure is simple, Home Assistant compatible, and follows the reference project's pattern. The `set/` prefix clearly separates commands from state.

### 7. DIYLess shield pin defaults

The DIYLess Thermostat Shield for ESP32 uses:
- **OT Input pin**: GPIO 21
- **OT Output pin**: GPIO 22

These are configurable via NVS in case of different wiring. OLED I2C uses default ESP32 pins (SDA=GPIO 19, SCL=GPIO 23). Encoder pins are configurable (defaults: CLK=GPIO 16, DT=GPIO 17, SW=GPIO 5).

## Risks / Trade-offs

- **[Single master]** → The ESP32 replaces the BM remote entirely. If the ESP32 loses power or crashes, there is no ventilation control. Mitigation: watchdog timer, auto-restart, and the CWL may fall back to a default mode without a master.
- **[OpenTherm timing sensitivity]** → The ihormelnyk library uses interrupts for timing-critical Manchester encoding. Heavy WiFi/MQTT activity could theoretically cause timing issues. Mitigation: OpenTherm communication happens in 1-second bursts with idle time between; WiFi operations run between OpenTherm exchanges.
- **[TSP scan time]** → Full TSP scan takes ~5-6 minutes (31 indices × 11s cycle). Mitigation: cache TSP values and only refresh periodically; configuration TSPs rarely change.
- **[Unknown data IDs]** → Some V/H data IDs (78, 79, 81, 83-85, 87, 88) haven't been tested. Mitigation: probe once at startup, log responses, skip IDs that return Unknown-DataId.
