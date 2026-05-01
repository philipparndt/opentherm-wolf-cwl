## 1. Add inverted header helper

- [ ] 1.1 Add `draw_header_inverted()` function in `display.rs` that draws a filled white rectangle (0,0 to 127,11) and renders the title text in BinaryColor::Off

## 2. Update home screen rendering

- [ ] 2.1 Modify the `else` branch of `draw_home()` to call `draw_header_inverted(d, "Schedule")` when `schedule_active && !schedule_override`, `draw_header(d, "Manual")` when `schedule_override`, and `draw_header(d, "Ventilation")` otherwise
- [ ] 2.2 Remove the `ind` variable and " M"/" S" suffix from the info line format string — display only percentage and bypass mode

## 3. Verify

- [ ] 3.1 Build the project with `cargo build` and confirm no compile errors
