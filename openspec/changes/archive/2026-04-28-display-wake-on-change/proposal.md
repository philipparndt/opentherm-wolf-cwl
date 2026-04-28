## Why

The OLED display enters standby after 5 minutes of inactivity. When the ventilation level or bypass mode is changed remotely (via MQTT or the scheduler), the user has no visual feedback that the change took effect. The display should wake up automatically so the new state is immediately visible.

## What Changes

- When `requestedVentLevel` is changed (via MQTT, scheduler, or timed-off activation/cancellation), call `displayWake()` so the display lights up for another 5 minutes.
- When `requestedBypassOpen` is changed (via MQTT or scheduler), call `displayWake()` so the display lights up for another 5 minutes.
- No changes to the existing encoder/web-triggered wake behavior or the 5-minute standby timeout.

## Capabilities

### New Capabilities

_None_ -- this change uses the existing `displayWake()` mechanism.

### Modified Capabilities

_None_ -- no spec-level behavior changes, this is an implementation enhancement.

## Impact

- **esp32/src/mqtt_client.cpp**: Add `displayWake()` calls in the level and bypass MQTT handlers.
- **esp32/src/scheduler.cpp**: Add `displayWake()` calls when the scheduler sets a new ventilation level, activates/cancels timed off, or changes bypass state.
- **esp32/src/display.h**: Ensure `displayWake()` is declared for external use (already declared).
