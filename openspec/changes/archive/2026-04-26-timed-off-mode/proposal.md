## Why

Turning ventilation to "Off" is dangerous for building health — prolonged off periods cause humidity buildup and poor air quality. Currently, selecting Off via the encoder just sets level=0 with no time limit and no schedule interaction. Users need a safe way to temporarily disable ventilation (e.g., during noisy work, parties with open windows) with an automatic return to normal operation.

## What Changes

- Selecting "Off" via encoder or MQTT becomes a **timed off** mode: a two-stage process where the user first selects Off, then rotates the dial to set the duration in hours (1-99h)
- When timed off is active, all ventilation schedules are suspended
- After the timer expires, the ventilation returns to Normal (level 2) and schedules are re-enabled
- The OLED home page shows a countdown during timed off
- MQTT commands can also trigger timed off with a duration
- The web UI shows timed off status and allows cancellation

## Capabilities

### New Capabilities
- `timed-off-mode`: Multi-stage off mode with duration selection (1-99h), schedule suspension, automatic return to Normal, countdown display, MQTT and web UI integration

### Modified Capabilities

(none)

## Impact

- **Modified files**: `display.cpp` (two-stage edit mode for Off, countdown display), `scheduler.cpp/.h` (suspend/resume schedules), `mqtt_client.cpp` (timed off command + status), `webserver.cpp` (timed off API + cancel), `ot_master.h` (timed off state)
- **Web UI**: Show timed off status with cancel button on Status tab
