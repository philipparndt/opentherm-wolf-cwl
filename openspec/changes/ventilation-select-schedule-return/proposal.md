## Why

The ventilation level selection is unintuitive in two ways:
1. There is no way to explicitly return to schedule-controlled mode from the rotary encoder UI — you can only select a manual level (Off/Reduced/Normal/Party), but never say "go back to schedule"
2. When you manually select a level while a schedule is active, the override gets cleared on the next schedule transition (time slot change), snapping back to the schedule even though the user explicitly chose something different

## What Changes

- Add a "Schedule" option to the rotary encoder level selection cycle (Off → Reduced → Normal → Party → Schedule → Off…) that clears the manual override and returns control to the active schedule
- When the user explicitly selects a manual level, that override persists until the user explicitly selects "Schedule" again (do not auto-clear on schedule transitions)
- If no schedule is active, selecting "Schedule" falls back to the configured default level

## Capabilities

### New Capabilities
- `schedule-return-option`: Adds a "Schedule" selection in the encoder edit cycle and makes manual overrides persist until explicitly cancelled

### Modified Capabilities

## Impact

- `esp32-rust/src/display.rs` — edit value cycling logic (`adjust_edit_value`) and exit logic (`exit_edit_mode`), display of the new option
- `esp32-rust/src/scheduler.rs` — remove the "clear override on schedule transition" behavior
- `esp32-rust/src/cwl_data.rs` — potentially extend `VentLevel` enum or use a separate sentinel value
