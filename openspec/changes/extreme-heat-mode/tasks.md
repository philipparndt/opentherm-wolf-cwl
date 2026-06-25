## 1. Config & state

- [x] 1.1 Add `extreme_heat_enabled: bool` to `AppConfig` with a default of `false`, and load/save it through the existing NVS config path (`/api/config` GET/POST + backup/restore)
- [x] 1.2 Add Extreme Heat fields to `AppStateInner`: `eh_current_level: u8`, `eh_last_change_epoch: i64`, and a bounded change-event ring buffer (e.g. `eh_events: VecDeque<(i64 /*epoch*/, u8 /*level*/)>` capped at ~64)
- [x] 1.3 Define named threshold/dwell constants (`EH_OFF_DELTA = 0.5`, `EH_PARTY_DELTA = -1.0`, `EH_DWELL_SECS = 900`)

## 2. Control logic (`extreme_heat.rs`)

- [x] 2.1 Create `esp32/src/extreme_heat.rs` and register it in `main.rs`
- [x] 2.2 Implement `select_level(supply: f32, exhaust: f32) -> VentLevel` mapping `delta = supply − exhaust` to the bands from the spec (Off / Reduced / Normal / Party)
- [x] 2.3 Implement the update step: skip when disabled, when epoch is invalid (`<= 1_700_000_000`), or when supply/exhaust temps are missing/invalid
- [x] 2.4 Apply the 15-minute dwell: only change `requested_vent_level` when the selected level differs AND `now_epoch − eh_last_change_epoch >= EH_DWELL_SECS`; on change, update `eh_current_level`, reset `eh_last_change_epoch`, and append an event
- [x] 2.5 Respect timed-off (do not raise level while `timed_off_active`)
- [x] 2.6 Handle manual override: when the user changes the level while the mode is enabled, adopt it as `eh_current_level` and reset the dwell timer
- [x] 2.7 Suppress time-of-day schedule effects while the mode is enabled (mutually exclusive ownership of `requested_vent_level`)
- [x] 2.8 Call the update step from the main loop / scheduler tick (~once per minute)

## 3. Backend API

- [x] 3.1 Add `GET /api/history` in `webserver.rs` returning supply & exhaust channels as JSON (per-bucket `{min,max}` oldest→newest, plus `bucketMs` and `newestBucketEpoch`/age)
- [x] 3.2 Add change-event data (`{epoch, level}[]`) — either `GET /api/heat-events` or an `events` array on `/api/history`
- [x] 3.3 Extend `/api/status` with Extreme Heat state (`enabled`, `currentLevel`, `lastChangeEpoch`)

## 4. Web UI

- [x] 4.1 Add API client functions in `web/src/api.ts` for the history + events endpoints and the new status fields
- [x] 4.2 Create a Preact SVG chart component plotting supply and exhaust series over time with a labelled temperature axis and distinct colours (existing CSS accent vars)
- [x] 4.3 Draw vertical markers at each change event, annotated with the selected level name (Off/Reduced/Normal/Party)
- [x] 4.4 Wire the chart into the status view and refresh it on the existing poll cadence (no full reload)
- [x] 4.5 Add an enable/disable toggle for Extreme Heat mode that posts to `/api/config`, reflecting current state

## 5. Verification

- [x] 5.1 Unit-test `select_level` band boundaries (including `delta = 0`, `+0.5`, `−1.0`) and the dwell gating logic
- [ ] 5.2 Manually verify in simulate-ot mode: temps crossing thresholds change the level only after the dwell, and markers appear on the web graph at each change _(requires flashing the esp32 / simulator — no xtensa+esp-idf toolchain in this environment)_
- [ ] 5.3 Verify disabled mode is fully inert (schedules/manual/timed-off unchanged) and that enabled state + config survive reboot _(requires a device — same toolchain limitation)_
- [x] 5.4 Rebuild the web bundle (Vite → `esp32/data/`); the SPIFFS image is packed from `data/` by `make build-fs`/`make flash` at flash time
