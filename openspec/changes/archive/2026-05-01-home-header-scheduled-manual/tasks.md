## 1. Add i18n strings

- [x] 1.1 Add `scheduled` and `manual` fields to the `Strings` struct in `i18n.rs` with EN values "Scheduled"/"Manual" and DE values "Zeitplan"/"Manuell"

## 2. Update home page header

- [x] 2.1 In `draw_home()` normal display branch, replace `draw_header(d, s.ventilation)` with mode-aware logic: `schedule_override` → `s.manual`, `schedule_active` → `s.scheduled`, else → `s.ventilation`
- [x] 2.2 Remove the `ind` variable and " M"/" S" suffix from the info line format string

## 3. Verify

- [x] 3.1 Run `cargo check` to confirm compilation
