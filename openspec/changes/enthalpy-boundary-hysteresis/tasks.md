## 1. Decision hysteresis (`esp32/src/humidity.rs`)

- [x] 1.1 Add constants `H_COOL_ON = -1.0` and `H_COOL_OFF = 0.0` next to `H_OFF`/`H_PARTY`, with a doc comment explaining the neutral band around `Δh = 0`
- [x] 1.2 Change `enthalpy_level(inp)` to `enthalpy_level(inp, was_cooling)`: keep Off (`> H_OFF`) and Party (`< H_PARTY`); in between, `cooling = if was_cooling { dh < H_COOL_OFF } else { dh < H_COOL_ON }`; map `cooling` → `(Normal, true)` else `(Reduced, false)`
- [x] 1.3 Unit tests: not-cooling holds Reduced for `Δh ∈ (H_COOL_ON, H_COOL_OFF]`; not-cooling enters Normal below `H_COOL_ON`; cooling holds Normal for `Δh ∈ [H_COOL_ON, H_COOL_OFF)`; cooling releases to Reduced above `H_COOL_OFF`; `< H_PARTY` still Party; `> H_OFF` still Off
- [x] 1.4 Regression test reproducing the live event: indoor h≈63.2, outdoor h 63.5→63.3 while `was_cooling=false` stays Reduced/not-cooling (no flip to cooling)

## 2. Wire cooling state + hold reason (`esp32/src/extreme_heat.rs`)

- [x] 2.1 In `decide`, derive `was_cooling = st.eh_current_reason == Reason::CoolingAssist` and pass it to `enthalpy_level`; `decide` now also returns `Option<dh>`
- [x] 2.2 Compute the hold cause in `update()`: decided level == held level and `deadband_holding(dh, level)` → `HoldReason::Deadband`; a *different* level decided but `now_epoch - eh_last_change_epoch < EH_DWELL_SECS` → `HoldReason::Dwell` (+ `eh_pending_level`); else `None`
- [x] 2.3 Store the hold cause (`eh_hold`) and `eh_pending_level` in `app_state`; clear on `record_change`, manual override, and when extreme-heat doesn't own the level
- [x] 2.4 Update the existing tests/call sites that invoke `enthalpy_level` with one argument

## 3. API (`esp32/src/webserver.rs`, `esp32/src/app_state.rs`)

- [x] 3.1 Add `HoldReason` enum + `eh_hold`/`eh_pending_level` to `app_state`; expose `holdReason` (`"none"|"deadband"|"dwell"`), `pendingLevel`, `dwellRemainingSecs` on `/api/status`. (Δh is derivable in the UI from the existing `indoor/outdoorEnthalpy`, so no separate field.)
- [x] 3.2 `dwellRemainingSecs` is computed live in the status handler from `eh_last_change_epoch`; the hold fields are set at the same per-minute evaluation as the reason

## 4. Web explainer (`esp32/web/src/StatusTab.tsx`, `translations.ts`)

- [x] 4.1 Render a "holding because …" line: deadband → "outdoor & indoor energy within the neutral band (Δh …)"; dwell → "would switch to {level} in {mm:ss} (dwell)"
- [x] 4.2 Hold notice is rendered distinctly from the active-rule line so a held decision doesn't read as an imminent switch
- [x] 4.3 Relabel the outdoor/indoor figures (via `aggregateHint`) so RH (max-RH sensor) and AH/h (wettest sensor @ aggregate T) are not presented as one consistent air mass
- [x] 4.4 Add translation keys (`holdDeadband`, `holdDwell`) in EN + DE
- [x] 4.5 Update `StatusTab.test.tsx` for the hold-reason rendering (deadband Δh, dwell countdown, none)

## 5. Boot robustness & actual-level display (from live-device findings)

- [x] 5.1 `humidity.rs`: `aggregate_side` returns `None` when no fresh sensor temperature exists and the inlet fallback is implausible, instead of fabricating an air state (+ unit test)
- [x] 5.2 `webserver.rs`: gate the derived humidity/enthalpy block on `cwl_data.connected`, so the explainer never shows a boot-transient air state (e.g. the observed indoor h ≈ 37.9)
- [x] 5.3 `cwl_data.rs`: add `VentLevel::from_relative_pct` (ID 77 % → level); reuse it in `ot_master.rs` (initial-level inference) and expose `ventilation.actualLevel` on `/api/status`
- [x] 5.4 `StatusTab.tsx`: highlight the unit's **actual** level (`actualLevel`) at the top, not the commanded one; clear the pending spinner on `actualLevel`
- [x] 5.5 `StatusTab.tsx` + `translations.ts`: show a "waiting for the unit" state when disconnected (instead of "data stale"); add `reasonText` for `schedule`/`reboot` so the explanation is never blank
- [x] 5.6 `StatusTab.test.tsx`: actual-vs-commanded highlight, and the waiting-for-unit note

## 6. Verification

- [ ] 6.1 `cargo +esp build --release` builds clean _(NOT RUN: this environment has no ESP-IDF / Xtensa toolchain — `IDF_PATH` empty, `esp-idf-sys` rejects the host target. Must be built on a device-capable machine.)_
- [x] 6.2 Web `tsc --noEmit` + `vite build` clean; `vitest` 26/26 pass (hold-notice, actual-level, waiting-for-unit). _Rust unit tests are written but cannot run here (same toolchain gap as 6.1); run `cargo test` on the ESP-capable host._
- [ ] 6.3 Manual: drive a Δh slowly across zero and confirm the level/reason hold through the neutral band with the deadband notice; force a pending switch and confirm the dwell countdown; reboot and confirm the explainer shows "waiting for the unit" (no h≈38 transient) and the top shows the actual level _(requires device + broker)_
