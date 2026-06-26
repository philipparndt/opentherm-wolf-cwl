## 1. Control logic (`extreme_heat.rs`)

- [x] 1.1 Add a named hysteresis constant `EH_BYPASS_HYST = 0.5`
- [x] 1.2 Implement `select_bypass(supply: f32, exhaust: f32, current_open: bool) -> bool`: open when `supply < exhaust − EH_BYPASS_HYST`, closed when `supply > exhaust + EH_BYPASS_HYST`, else hold `current_open`
- [x] 1.3 In the update step, after the live supply/exhaust temps are read and gated by the same readiness checks (mode enabled, not timed-off, NTP synced, connected), set `requested_bypass_open` from `select_bypass(...)`; log and flag a publish only when it changes
- [x] 1.4 Touch only `requested_bypass_open`, never the persisted `config.bypass_open`, so automatic toggling does not write NVS

## 2. Scheduler ownership (`scheduler.rs`)

- [x] 2.1 Gate the calendar bypass schedule on `!extreme_heat_enabled` so it yields the damper while the mode is enabled and resumes when disabled

## 3. Verification

- [x] 3.1 Unit-test `select_bypass`: open when clearly cooler, closed when clearly hotter, hold within the hysteresis band, and the exact-band-edge boundary
- [x] 3.2 `cargo check` against the esp32 target compiles cleanly
- [ ] 3.3 Manually verify in simulate-ot mode: supply rising above exhaust closes the bypass, falling below opens it, and readings near equality hold the damper _(requires flashing the esp32 / simulator — no xtensa+esp-idf toolchain in this environment)_
- [ ] 3.4 Verify the calendar bypass schedule no longer fights the mode while enabled, and resumes after disabling _(requires a device — same toolchain limitation)_
