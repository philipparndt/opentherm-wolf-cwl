## 1. History module — types and storage

- [x] 1.1 Create `src/history.rs` with the constants `HISTORY_SLOTS = 128`, `BUCKET_MS = 675_000`, `SAMPLE_INTERVAL_MS = 1_000`
- [x] 1.2 Define `Bucket { min: f32, max: f32 }` (`Debug, Clone, Copy`)
- [x] 1.3 Define `Channel { slots: [Option<Bucket>; HISTORY_SLOTS], head: usize }` with `Channel::new()` returning all-`None` and `head = 0`
- [x] 1.4 Define `TempHistory { outdoor: Channel, indoor: Channel, delta: Channel, bucket_start_ms: u32, last_sample_ms: u32 }` and `TempHistory::new()`

## 2. History module — sampling

- [x] 2.1 Add `fn fold(channel: &mut Channel, value: f32)` — initialise `slots[head]` to `Some(Bucket { min: value, max: value })` if `None`, otherwise update min/max in place
- [x] 2.2 Add `fn advance(channel: &mut Channel)` — set `slots[head] = None` (clear stale bucket about to be reused) **then** `head = (head + 1) % HISTORY_SLOTS`
- [x] 2.3 Implement `TempHistory::sample(&mut self, now_ms: u32, data: &CwlData) -> bool`:
  - Return `false` early if `now_ms.wrapping_sub(self.last_sample_ms) < SAMPLE_INTERVAL_MS`
  - Set `self.last_sample_ms = now_ms`
  - If `now_ms.wrapping_sub(self.bucket_start_ms) >= BUCKET_MS`: call `advance` on all three channels, set `bucket_start_ms = now_ms`, mark `rolled = true`
  - Always call `fold(outdoor, data.supply_inlet_temp)` and `fold(indoor, data.exhaust_inlet_temp)`
  - Call `fold(delta, data.supply_outlet_temp - data.supply_inlet_temp)` **only if** `data.supports_id81`
  - Return `rolled`
- [x] 2.4 Add `Channel::min_max(&self) -> Option<(f32, f32)>` — fold across populated slots, returning `None` if all `None`

## 3. App-state wiring

- [x] 3.1 In `src/app_state.rs`: import the new module, add `pub temp_history: history::TempHistory` to `AppStateInner`, initialise in `AppStateInner::new` via `TempHistory::new()`
- [x] 3.2 In `src/main.rs`: add `mod history;` next to the other module declarations

## 4. Main-loop integration

- [x] 4.1 In `src/main.rs` main loop body, after the existing `Watchdog` block and before the "Virtual encoder from web UI" block: lock `state`, call `state.temp_history.sample(now_ms, &state.cwl_data)`, capture the returned `rolled` bool, drop the lock
- [x] 4.2 If `rolled` is true, set `display_dirty.store(true, Ordering::Relaxed)` so the chart redraws on slot rollover

## 5. Display — Page enum and routing

- [x] 5.1 In `src/display.rs`: change `pub const PAGE_COUNT: usize = 6;` to `9`
- [x] 5.2 Extend `enum Page` with `OutdoorHistory`, `IndoorHistory`, `DeltaHistory` inserted between `TempIn` and `Status` (preserving discriminant order: Home=0, Bypass=1, TempIn=2, OutdoorHistory=3, IndoorHistory=4, DeltaHistory=5, Status=6, System=7, Settings=8)
- [x] 5.3 Update `Page::from_index` match arms to cover the 9 variants
- [x] 5.4 Add new arms to `render_content`'s `match page` for the three new pages, calling `draw_outdoor_history`, `draw_indoor_history`, `draw_delta_history`

## 6. Display — i18n strings

- [x] 6.1 In `src/i18n.rs` add fields to `Strings`: `outdoor_24h`, `indoor_24h`, `delta_24h`, `min_label`, `max_label`, `history_empty`, `celsius_unit`
- [x] 6.2 Populate `EN`: `"Outdoor 24h"`, `"Indoor 24h"`, `"Gain 24h"`, `"Min"`, `"Max"`, `"Collecting..."`, `"C"`
- [x] 6.3 Populate `DE`: `"Außen 24h"` (escaped), `"Innen 24h"`, `"Gewinn 24h"`, `"Min"`, `"Max"`, `"Sammle..."`, `"C"`

## 7. Display — chart rendering helper

- [x] 7.1 Add a private `fn draw_temp_chart(d, channel: &history::Channel, lang: Strings, empty_hint: &str)` in `src/display.rs`
- [x] 7.2 Inside the helper: call `channel.min_max()`; if `None`, render `empty_hint` centred in the chart area (y=24..56) and `--   --` for the min/max strip at y=12
- [x] 7.3 If `Some((lo, hi))`: render the numeric strip `"Min {lo:.1} C   Max {hi:.1} C"` at y=12 (small font, centred), then compute a stable y-scale:
  - `range = (hi - lo).max(0.5)`
  - `pad = (range - (hi - lo)) / 2.0`
  - effective `[lo - pad, hi + pad]` maps to `y = 56..24`
- [x] 7.4 For each `i in 0..128` of the channel's slots: read `Channel::slot_at_column(i)` (helper that returns `slots[(head + i) % HISTORY_SLOTS]`, so column 0 is oldest and column 127 is newest). If `Some(b)`, draw `Line::new(Point::new(i as i32, scale(b.max)), Point::new(i as i32, scale(b.min))).into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1)).draw(d).ok();`
- [x] 7.5 Add `Channel::slot_at_column(&self, col: usize) -> Option<Bucket>` to history module

## 8. Display — per-page draw functions

- [x] 8.1 Add `fn draw_outdoor_history(d, st, lang)`: `draw_header(d, tr(lang).outdoor_24h)`, then `draw_temp_chart(d, &st.temp_history.outdoor, tr(lang), tr(lang).history_empty)`
- [x] 8.2 Add `fn draw_indoor_history` analogously, reading from `st.temp_history.indoor` with header `tr(lang).indoor_24h`
- [x] 8.3 Add `fn draw_delta_history` analogously, reading from `st.temp_history.delta` with header `tr(lang).delta_24h`. No special-case for "ID 81 unsupported" — the channel will simply be empty, so the empty-history hint already covers it

## 9. Verify

- [x] 9.1 `cargo check` passes (no feature flags) — verified with default `wifi,ot-ext,display-sh1106`
- [x] 9.2 `cargo check --features simulate-ot` passes — verified, the dev-mode preload path compiles
- [x] 9.3 Unit tests added in `history.rs` (fold, advance, min_max empty, min_max aggregate, slot_at_column ordering). Tests are compile-checked; they cannot be executed via `cargo test` on host because `esp-idf-sys` rejects non-ESP targets — same limitation as the existing `esp32/src/cwl_data.rs` tests
- [ ] 9.4 Build firmware (`cargo build --release`) and flash to a device; confirm: navigating past the existing `Intake` page reaches three new pages with the expected headers; chart starts empty with `Collecting...`; after running for >12 min the chart begins to populate columns; min/max strip updates on slot rollover *(requires hardware)*
- [ ] 9.5 Confirm no regressions on the existing 6 pages — dots indicator now shows 9 dots, navigation wraps correctly at boundaries, edit-mode pages (Home, Bypass, Settings) still work *(requires hardware)*
