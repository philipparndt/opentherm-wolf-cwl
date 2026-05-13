## 1. Extend edit level cycling

- [x] 1.1 In `display.rs` `adjust_edit_value()`, change the home page level cycling from `rem_euclid(4)` to `rem_euclid(5)` so values 0-4 are reachable
- [x] 1.2 Update the off-duration sub-stage backward rotation: when rotating back past 1h, set `edit_vent_level = 4` (Schedule) instead of 3 (Party) — or keep as Party depending on UX preference

## 2. Display the Schedule option

- [x] 2.1 In `draw_home()` edit mode branch, handle `edit_level == 4` by displaying "Schedule" as the level name (either extend `ventilation_level_name()` or handle inline)

## 3. Handle Schedule selection in exit_edit_mode

- [x] 3.1 In `exit_edit_mode()`, add a branch for `edit_vent_level == 4`: set `schedule_override = false`, cancel timed-off if active, and let the scheduler take control on next tick
- [x] 3.2 Ensure the Off sub-stage (duration selection) is NOT entered when selecting Schedule — only level 0 triggers duration sub-stage

## 4. Remove auto-clear of override on schedule transition

- [x] 4.1 In `scheduler.rs`, remove the block (around lines 127-134) that sets `schedule_override = false` when `match_index != self.last_active_index`

## 5. Verify

- [x] 5.1 Build with `cargo check` and confirm no compile errors (linker requires ESP-IDF hardware target, Rust compilation passes clean)
