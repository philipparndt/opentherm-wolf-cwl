## Why

The home screen header always shows "Ventilation" regardless of whether the current level is schedule-driven or manually set. This doesn't tell the user at a glance which mode is active. Changing the header to "Scheduled" or "Manual" makes the current control mode immediately obvious.

## What Changes

- Replace the static "Ventilation" header on the home page with a mode-aware header:
  - "Scheduled" when `schedule_active && !schedule_override`
  - "Manual" when `schedule_override`
  - "Ventilation" as fallback (no schedule active, no override)
- Add the new strings ("Scheduled"/"Manual" and German equivalents) to the i18n module
- Remove the " M"/" S" suffix from the info line since the header now communicates the mode

## Capabilities

### New Capabilities

### Modified Capabilities

## Impact

- `esp32/src/display.rs` — `draw_home()` header logic
- `esp32/src/i18n.rs` — new `scheduled` and `manual` string fields
