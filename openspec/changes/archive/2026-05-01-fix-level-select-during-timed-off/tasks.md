## 1. Fix cancel_timed_off

- [x] 1.1 In `scheduler.rs` `cancel_timed_off()`, remove the lines that set `st.requested_vent_level = 2` and `st.schedule_override = false`

## 2. Verify

- [x] 2.1 Run `cargo check` to confirm compilation
