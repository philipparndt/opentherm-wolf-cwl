## Why

When a ventilation level is selected (via encoder or web), there's a delay before the CWL unit confirms the new level via OpenTherm. During this time the display still shows the old confirmed level, giving no feedback that the switch is in progress. The user should see the requested level immediately with a visual indicator that it's pending confirmation.

## What Changes

- On the home screen, when `requested_vent_level != cwl_data.ventilation_level`, show the requested level with a ">" prefix (e.g. "> Reduced") instead of the confirmed level
- Once the CWL confirms (levels match), display reverts to normal (no prefix)

## Capabilities

### New Capabilities

### Modified Capabilities

## Impact

- `esp32/src/display.rs` — `draw_home()` normal display branch: compare requested vs confirmed level and format accordingly
