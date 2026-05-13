## 1. Show pending level with prefix

- [x] 1.1 In `draw_home()` normal display branch, compare `st.requested_vent_level` with `st.cwl_data.ventilation_level`. If different, format as `"> {level_name}"` using the requested level; otherwise show confirmed level as before.

## 2. Verify

- [x] 2.1 Run `cargo check` to confirm compilation
