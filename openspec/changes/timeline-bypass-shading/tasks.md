## 1. Bypass transition log (firmware state)

- [x] 1.1 Add a `BypassEvent { epoch: i64, open: bool }` type and a bounded `bypass_events: VecDeque<BypassEvent>` to `AppStateInner` (cap as a named const, e.g. `BYPASS_EVENT_CAPACITY`, generous for 24 h of toggling)
- [x] 1.2 Add a single chokepoint helper (e.g. `AppStateInner::set_bypass_open(open, epoch)`) that updates `requested_bypass_open` and, only on an actual change, appends a transition (oldest dropped when full); make all writers go through it
- [x] 1.3 Route the three writers through the helper: extreme-heat (`extreme_heat.rs`), the calendar bypass schedule (`scheduler.rs`), and the manual web/MQTT/encoder paths (`webserver.rs` / `mqtt.rs` / `display.rs`)

## 2. Expose & persist the bypass log

- [x] 2.1 Build a `bypass` JSON array of `{epoch, open}` in the `/api/history` handler (`webserver.rs`), mirroring how `events` is built from `eh_events`
- [x] 2.2 Extend `TempHistory::full_json` (or the handler) to include the `bypass` field in the `/api/history` payload
- [x] 2.3 Include the bypass transitions in the retained `persist/temp_history` MQTT snapshot (`snapshot_json`) and restore them in `restore_from_snapshot`; treat a missing field as "no shading data"

## 3. Decouple the chart component (frontend)

- [x] 3.1 Rename `web/src/ExtremeHeatChart.tsx` → `web/src/TimelineChart.tsx` and rename the exported component to `TimelineChart`
- [x] 3.2 Update the import/usage in `StatusTab.tsx`; confirm the chart renders independent of `status.extremeHeat.enabled` and that level-change markers/legend only appear when events exist

## 4. Bypass background shading (frontend)

- [x] 4.1 Add `bypass: { epoch: number; open: boolean }[]` to the `History` type in `api.ts`
- [x] 4.2 Build bypass spans from the transitions over the window: order by epoch, derive the left-edge state as the inverse of the earliest transition's `open`, and clamp span x-coords with the existing `eventX` epoch→x mapping
- [x] 4.3 Render one background `<rect>` per span (y `PAD_T..PAD_T+PLOT_H`), drawn first so it sits behind gridlines/series/markers; fill closed spans with a base gray and open spans with a slightly brighter gray
- [x] 4.4 Add two theme-aware CSS custom properties for the tones (e.g. `--chart-bg`, `--chart-bg-bypass`) in `style.css`
- [x] 4.5 Add a legend entry for the bypass shading and EN/DE strings in `translations.ts`

## 5. Verification

- [x] 5.1 `cargo check` against the esp32 target compiles cleanly; unit-test the transition-log helper (records only on change, drops oldest when full)
- [x] 5.2 `npm run build` (tsc + vite) succeeds and writes the bundle to `esp32/data/`
- [ ] 5.3 Manually verify in simulate-ot mode: toggling the bypass paints a brighter band for open spans aligned to the time axis, the leading span before the first transition is tinted correctly, and the timeline renders with Extreme Heat off _(requires flashing the esp32 / simulator — no xtensa+esp-idf toolchain in this environment)_
- [ ] 5.4 Verify the shading survives a reboot via the retained history snapshot _(requires a device — same toolchain limitation)_
