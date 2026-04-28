## Why

When running the emulator build (`SIMULATE_OT`), there is no visible indication that the device is using simulated data rather than a real Wolf CWL unit. This can lead to confusion during development and testing — someone looking at the OLED or web UI might think they're seeing real data.

## What Changes

- Show a "SIM" indicator on the OLED display so it's immediately obvious the firmware is in simulation mode.
- Include a `simulated` flag in the `/api/status` response and show a visual "Simulation" banner on the web UI.

## Capabilities

### New Capabilities

_None_ — this is a small cross-cutting indicator, not a new capability requiring a spec.

### Modified Capabilities

_None_

## Impact

- **esp32/src/display.cpp**: Draw a "SIM" label on the OLED (e.g., top-right corner) when `SIMULATE_OT` is defined.
- **esp32/src/webserver.cpp**: Add `simulated` field to the `/api/status` response.
- **esp32/web/src/**: Show a simulation banner/indicator when the API reports `simulated: true`.
