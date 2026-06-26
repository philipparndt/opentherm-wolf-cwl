## 1. Decision hysteresis (`esp32/src/humidity.rs`)

- [ ] 1.1 Add constants `H_COOL_ON = -1.0` and `H_COOL_OFF = 0.0` next to `H_OFF`/`H_PARTY`, with a doc comment explaining the neutral band around `Δh = 0`
- [ ] 1.2 Change `enthalpy_level(inp)` to `enthalpy_level(inp, was_cooling)`: keep Off (`> H_OFF`) and Party (`< H_PARTY`); in between, `cooling = if was_cooling { dh < H_COOL_OFF } else { dh < H_COOL_ON }`; map `cooling` → `(Normal, true)` else `(Reduced, false)`
- [ ] 1.3 Unit tests: not-cooling holds Reduced for `Δh ∈ (H_COOL_ON, H_COOL_OFF]`; not-cooling enters Normal below `H_COOL_ON`; cooling holds Normal for `Δh ∈ [H_COOL_ON, H_COOL_OFF)`; cooling releases to Reduced above `H_COOL_OFF`; `< H_PARTY` still Party; `> H_OFF` still Off
- [ ] 1.4 Regression test reproducing the live event: indoor h≈63.2, outdoor h 63.5→63.3 while `was_cooling=false` stays Reduced/not-cooling (no flip to cooling)

## 2. Wire cooling state + hold reason (`esp32/src/extreme_heat.rs`)

- [ ] 2.1 In `decide`, derive `was_cooling = matches!(st.eh_current_reason, Reason::CoolingAssist)` and pass it to `enthalpy_level`
- [ ] 2.2 Compute the hold cause in `update()`: when the decided level equals the held level because Δh is inside `[H_COOL_ON, H_COOL_OFF]` → `HoldReason::Deadband`; when a *different* level is decided but `now_epoch - eh_last_change_epoch < EH_DWELL_SECS` → `HoldReason::Dwell { remaining_secs, pending_level }`; else none
- [ ] 2.3 Store the hold cause (and pending level / dwell-remaining) in `app_state` for the status payload; clear it when a change is applied
- [ ] 2.4 Update the existing tests/call sites that invoke `enthalpy_level` with one argument

## 3. API (`esp32/src/webserver.rs`, `esp32/src/app_state.rs`)

- [ ] 3.1 Add `holdReason` (`null` | `"deadband"` | `"dwell"`), `dwellRemainingSecs`, and `pendingLevel` to `/api/status`, plus the current `Δh`
- [ ] 3.2 Ensure these reflect the same evaluation cadence as the existing reason fields

## 4. Web explainer (`esp32/web/src/StatusTab.tsx`, `translations.ts`)

- [ ] 4.1 Render a "holding because …" line: deadband → "outdoor & indoor energy within the neutral band (Δh …)"; dwell → "would switch to {level} in {mm:ss} (dwell)"
- [ ] 4.2 Suppress/adjust the active-rule wording so a held decision doesn't read as an imminent switch
- [ ] 4.3 Relabel the outdoor/indoor figures so RH (max-RH sensor) and AH/h (wettest sensor @ aggregate T) are not presented as one consistent air mass
- [ ] 4.4 Add translation keys for the new strings
- [ ] 4.5 Update `StatusTab.test.tsx` for the hold-reason rendering and the relabelled figures

## 5. Verification

- [ ] 5.1 `cargo +esp build --release` and web `tsc` + `vite` build clean
- [ ] 5.2 Unit tests in 1.3/1.4/2.x pass
- [ ] 5.3 Manual: drive a Δh slowly across zero (or replay the two readings) and confirm the level and reason hold through the neutral band, and the explainer shows the deadband hold; force a pending switch and confirm the dwell countdown shows _(requires device + broker)_
