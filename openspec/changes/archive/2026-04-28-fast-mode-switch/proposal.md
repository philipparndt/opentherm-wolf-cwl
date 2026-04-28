## Why

Switching ventilation modes via the display encoder feels sluggish. The cause is `saveConfig()` in `exitEditMode()` — it writes the entire configuration (WiFi, MQTT, pins, ventilation state, etc.) to NVS flash synchronously, blocking the main loop. NVS writes on ESP32 take tens of milliseconds per key, and the full config has ~20 keys.

Additionally, persisting every mode switch to NVS causes unnecessary flash wear. Only the "Off" mode (timed off) needs persistence so it survives reboots — other levels are transient and will be overridden by the scheduler anyway.

## What Changes

- Remove `saveConfig()` from the ventilation and bypass mode switch paths in `exitEditMode()`.
- Apply the mode change and update the display immediately (no NVS in the hot path).
- Only persist to NVS for timed-off activation (already handled by `activateTimedOff()` which uses the scheduler's own NVS persistence).
- For bypass, persist via a deferred/lightweight save of just the bypass key.

## Capabilities

### New Capabilities

_None_

### Modified Capabilities

_None_ — no spec-level behavior changes. The mode still switches, the display still updates, timed-off still survives reboots.

## Impact

- **esp32/src/display.cpp**: Remove `saveConfig()` calls from `exitEditMode()` for non-off ventilation level changes. For bypass, replace with a targeted NVS write.
- **esp32/src/config_manager.cpp / config_manager.h**: Optionally add a `saveVentilationState()` function that only writes the ventilation/bypass keys (not the full config).
