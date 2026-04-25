## Why

The ESP32 is the sole OpenTherm master controlling the Wolf CWL 300 ventilation unit. If the firmware hangs (WiFi driver lock, OpenTherm interrupt deadlock, heap exhaustion, runaway loop), the CWL receives no master communication and may revert to a degraded default mode — or lose ventilation entirely. The device needs self-healing capability to guarantee near-100% uptime without manual intervention.

## What Changes

- Enable ESP32 hardware task watchdog timer (TWDT) on the main loop to force a hardware reset if `loop()` stops executing
- Add software liveness checks for critical subsystems: OpenTherm communication, WiFi connectivity, and MQTT connection
- Auto-restart the ESP32 if the OpenTherm bus has been unresponsive for an extended period (configurable timeout)
- Track and persist reboot reason and crash count via NVS for diagnostics
- Publish watchdog/health telemetry via MQTT (uptime, reboot reason, crash count, last OT response age)
- Show reboot reason on the OLED system page

## Capabilities

### New Capabilities
- `watchdog`: Hardware watchdog (TWDT) on the main loop, software liveness monitors for OpenTherm/WiFi/MQTT, automatic restart on prolonged failure, reboot diagnostics

### Modified Capabilities

(none)

## Impact

- **Modified files**: `main.cpp` (TWDT init + feed), `status.cpp` (reboot reason tracking), `mqtt_client.cpp` (health telemetry), `display.cpp` (system page reboot info), `config_manager.cpp/.h` (crash counter in NVS)
- **New file**: `watchdog.cpp/.h` (liveness monitor logic)
- **Dependencies**: ESP-IDF TWDT API (`esp_task_wdt.h`) — already available in Arduino-ESP32 framework, no new library needed
