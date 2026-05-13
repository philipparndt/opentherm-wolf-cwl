## Why

When timed-off is active and the user selects a manual level (e.g. "Reduced"), the selection is immediately overwritten. The display code correctly sets `requested_vent_level` and `schedule_override = true`, but then `cancel_timed_off` in the scheduler resets both to Normal/no-override. The user's explicit selection is lost.

## What Changes

- Fix `cancel_timed_off()` in `scheduler.rs` to only deactivate the timer without overwriting the ventilation level or override state
- The caller (display exit_edit_mode) already sets the correct level and override state before triggering the cancel

## Capabilities

### New Capabilities

### Modified Capabilities

## Impact

- `esp32/src/scheduler.rs` — `cancel_timed_off()` method: remove the lines that set `requested_vent_level = 2` and `schedule_override = false`
