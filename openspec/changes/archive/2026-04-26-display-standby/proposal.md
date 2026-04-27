## Why

The OLED display runs continuously, which causes burn-in on OLED panels over time and is unnecessary when nobody is looking at it. The display should turn off after 5 minutes of inactivity to extend its lifespan — but stay on while the timed off countdown is active since that's useful information. Any encoder interaction wakes it up, but the wake-up input itself is consumed (not forwarded as a page change or edit action).

## What Changes

- Display turns off after 5 minutes of no encoder activity
- Display stays on while timed off countdown is active (useful live information)
- Any encoder rotation or button press wakes the display; that input is discarded
- Subsequent encoder inputs after wake-up work normally

## Capabilities

### New Capabilities

(none — display behavior enhancement)

### Modified Capabilities

(none)

## Impact

- **Modified files**: `display.cpp/.h` (standby state, wake/sleep logic), `encoder.cpp` (consume input on wake)
- No web UI, MQTT, or scheduler changes
