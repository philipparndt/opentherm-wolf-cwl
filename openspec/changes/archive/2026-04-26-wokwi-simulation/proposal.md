## Why

The hardware (ESP32 + DIYLess shield + OLED + encoder) hasn't arrived yet. Development and testing of the web UI, MQTT, schedules, display pages, and encoder navigation should be possible without physical hardware. Wokwi is an ESP32 simulator that integrates with PlatformIO and can visually simulate the SH1106 OLED display and KY-040 rotary encoder in a browser.

## What Changes

- Add a `SIMULATE_OT` compile flag that replaces real OpenTherm communication with fake CWL sensor data (realistic temperatures, ventilation values, status flags, TSP registers)
- Add a `[env:esp32-sim]` PlatformIO environment for simulation builds
- Add Wokwi configuration files (`wokwi.toml`, `diagram.json`) with ESP32, SH1106 OLED, and KY-040 encoder wired up
- Add a `make simulate` target that builds and launches the Wokwi simulation

## Capabilities

### New Capabilities

(none — simulation tooling, no new firmware features)

### Modified Capabilities

(none)

## Impact

- **New files**: `wokwi.toml`, `diagram.json`
- **Modified files**: `platformio.ini` (new env), `ot_master.cpp` (simulation mode), `Makefile` (simulate target)
- **No impact on production firmware** — `SIMULATE_OT` is only set in `esp32-sim` environment
