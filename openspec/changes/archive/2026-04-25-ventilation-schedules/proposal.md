## Why

Currently the ventilation level can only be changed manually — via MQTT, the rotary encoder, or the web UI. Users want automatic time-based control: reduced ventilation at night, normal during the day, party mode in the evening. Without schedules, the user must either leave the level fixed or rely on Home Assistant automations, which fail if MQTT is down.

## What Changes

- Add a schedule engine that evaluates configured time slots against the current time and day-of-week, and automatically sets the ventilation level
- Store up to 8 schedule entries in NVS, each with: start time (HH:MM), end time (HH:MM), ventilation level (0-3), and active days (bitmask Mon-Sun)
- Add a schedule configuration page in the Preact web UI where users can create, edit, and delete schedule entries
- Add an API endpoint for CRUD operations on schedules
- Add a "schedule active" indicator on the OLED home page
- Allow manual override: MQTT or encoder changes temporarily override the schedule until the next schedule transition
- Publish the current schedule state via MQTT

## Capabilities

### New Capabilities
- `ventilation-schedules`: Time-based ventilation level control with per-day scheduling, manual override, NVS persistence, web UI configuration, and MQTT integration

### Modified Capabilities

(none)

## Impact

- **New files**: `scheduler.cpp/.h` (schedule engine + NVS storage)
- **Modified files**: `main.cpp` (call scheduler in loop), `config_manager.cpp/.h` (schedule data structures), `webserver.cpp` (schedule API endpoints), `mqtt_client.cpp` (schedule state publishing), `display.cpp` (schedule indicator on home page)
- **Web UI**: New "Schedules" tab in the Preact app
- **NVS**: New keys for schedule storage (up to 8 entries)
- **Dependencies**: None new — uses existing NTP time from Arduino-ESP32
