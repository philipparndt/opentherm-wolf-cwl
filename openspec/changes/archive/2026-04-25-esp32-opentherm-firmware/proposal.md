## Why

The Wolf CWL 300 ventilation unit uses a proprietary BM remote control that communicates via OpenTherm. ESPHome's OpenTherm component only supports boiler data IDs and cannot control ventilation/heat-recovery units (data IDs 70-91). A custom ESP32 firmware is needed to integrate the CWL into a smart home via MQTT, with local control through an OLED display and rotary encoder.

## What Changes

- New ESP32 PlatformIO firmware that acts as an OpenTherm master via the DIYLess Thermostat Shield, replacing the BM remote
- Polls all supported Wolf CWL data IDs in an ~11-second cycle matching the original BM remote pattern
- Controls ventilation level (Off/Reduced/Normal/Party), bypass damper, and filter reset
- Publishes all sensor data (temperatures, ventilation %, status, faults, TSP registers) to MQTT for Home Assistant
- Accepts MQTT commands to change ventilation level and bypass state
- SSD1306 128x64 OLED display with rotary encoder for local monitoring and control
- Web UI (Preact) for WiFi setup, MQTT configuration, and system monitoring
- OTA firmware updates, AP mode for initial setup, mDNS discovery

## Capabilities

### New Capabilities
- `opentherm-master`: OpenTherm master communication with Wolf CWL via DIYLess shield — polling cycle, ventilation control, bypass control, TSP register scanning
- `mqtt-integration`: MQTT publishing of all sensor values and subscription for commands (ventilation level, bypass, filter reset)
- `oled-display`: SSD1306 OLED display with multiple pages showing ventilation state, temperatures, status, and faults
- `rotary-encoder`: KY-040 rotary encoder for page navigation and ventilation level control
- `network-config`: WiFi AP mode setup, NVS configuration persistence, web server with JWT auth, OTA updates, mDNS

### Modified Capabilities

(none — this is a new firmware project)

## Impact

- **New project files**: PlatformIO project under `esp32/` directory with full firmware source, web UI, and build system
- **Dependencies**: Arduino framework, OpenTherm library (ihormelnyk), PubSubClient, ESPAsyncWebServer, ArduinoJson, U8g2 (OLED), ESP32Encoder
- **Hardware**: ESP32 dev board + DIYLess Master OpenTherm Shield + SSD1306 OLED + KY-040 encoder
- **Replaces**: BM remote control on the OpenTherm bus (only one master allowed)
