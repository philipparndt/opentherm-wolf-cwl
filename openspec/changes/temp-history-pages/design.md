## Context

`AppStateInner.cwl_data` already holds live temperatures, refreshed once per second by `OtMaster` on its own thread (see `src/ot_master.rs`). The OLED renderer in `src/display.rs` is dirty-flag driven: any thread that mutates display-visible state flips `DisplayDirty`, and `Display::update()` re-renders only when the flag is set. The render path runs `match page { … }` against a `Page` enum and an indexed dots strip.

There is no existing time-series store anywhere in the firmware — MQTT publishing is fire-and-forget. NVS is reserved for config + schedules; using it for sensor history would burn the flash for no panel benefit.

The display is 128×64 px, 1 bpp, with a fixed ~12 px header and a 5-px-tall dots row at y=59, leaving roughly y=14..56 (~42 px) for content per page.

## Goals / Non-Goals

**Goals:**
- Show the last 24 h of three temperature signals (outdoor / indoor / delta) on three new pages.
- Surface the 24 h min and max for each signal.
- Render a chart that fits the existing page layout without scrolling or controls.
- Zero flash wear: history is in RAM and reset on reboot.

**Non-Goals:**
- Persisting history across reboots (NVS wear, NTP/timezone complications — not worth it for a glance feature).
- Scrolling / zooming the chart, or selecting time ranges.
- Publishing the history over MQTT / HTTP (subscribers can already build this externally).
- Backfilling history from MQTT or anywhere else after reboot.
- Changing the live `TempIn` (Intake) page — it stays as-is.

## Data Model

```rust
// src/history.rs
pub const HISTORY_SLOTS: usize = 128;            // chart-width-aligned
pub const BUCKET_MS: u32      = 24 * 60 * 60 * 1000 / HISTORY_SLOTS as u32; // 675_000 ms (~11.25 min)
pub const SAMPLE_INTERVAL_MS: u32 = 1_000;       // main-loop folds once per second

#[derive(Debug, Clone, Copy)]
pub struct Bucket {
    pub min: f32,
    pub max: f32,
}

#[derive(Debug)]
pub struct Channel {
    pub slots: [Option<Bucket>; HISTORY_SLOTS], // None until first sample lands
    pub head: usize,                            // next slot to write
}

#[derive(Debug)]
pub struct TempHistory {
    pub outdoor: Channel,
    pub indoor:  Channel,
    pub delta:   Channel,                       // None whenever supports_id81 = false
    bucket_start_ms: u32,                       // when slot `head` started
    last_sample_ms:  u32,
}
```

`Channel::slots` is a fixed array (not a `Vec`) so the structure has a known footprint: 128 × 2 × 4 B per channel = 1 KB; three channels = 3 KB; plus `Option` discriminants ≈ 3.5 KB total. This fits the existing firmware budget comfortably.

`head` is the index of the *current* slot being aggregated. After `BUCKET_MS` of wall-clock time it advances (modulo `HISTORY_SLOTS`) and the new slot starts as `None`. The chart treats `None` as "no data drawn", which gives the cold-start ramp and rolling-window discard for free.

## Decisions

### 1. RAM-only, no NVS persistence

**Choice:** History lives in `AppStateInner` and dies with the process.

**Rationale:** The user signed off on this in the proposal questions. The flash wear of writing 3 channels × 128 slots multiple times per hour is unjustified for a feature that is "look at the panel". A fresh boot showing an empty chart that fills in over the next 24 h is acceptable.

### 2. Bucketing happens on the main loop, not on the OT thread

**Choice:** `state.temp_history.sample(now_ms, &state.cwl_data)` is called once per main-loop iteration from `src/main.rs`, throttled to once per second.

**Rationale:** Keeps `ot_master.rs` focused on protocol. The main loop already locks `state` for the encoder / network / persist checks, so adding one more borrow there is free. Sampling at 1 Hz oversamples relative to the OT polling rate (also ~1 Hz) — duplicates inside a bucket are folded into min/max, so there is no bias.

**Alternative considered:** Sample from the OT thread right after temperatures update. Rejected — would couple the new module to the OT path and require a separate dirty flag to wake the display.

### 3. Bucket aggregation: min/max only, no average

**Choice:** Each `Bucket` stores `(min, max)` over its window. No mean / count / sum.

**Rationale:** The chart draws a vertical bar per column from `min` to `max`, which doubles as the visual mean for narrow ranges. Adding mean would inflate per-bucket size by 50% and the renderer has no use for it (a single-pixel "mean line" on top of the bar is illegible on 1-bpp). Page-level "24 h min" and "24 h max" are scans over `slots[*].min` and `slots[*].max` respectively — O(128) on each redraw, negligible.

### 4. Delta channel definition: `supply_outlet − supply_inlet`

**Choice:** "Heat-recovery gain" = `cwl_data.supply_outlet_temp - cwl_data.supply_inlet_temp`, sampled only when `cwl_data.supports_id81` is true.

**Rationale:** This is the temperature the heat exchanger adds to the incoming fresh air — the most useful single number for "is the HRV doing its job". Indoor / outdoor delta would be informative but is dominated by the home's thermostat behaviour, not the unit's. While ID 81 is unsupported the page shows the empty-history hint, falling back gracefully on Wolf controllers that don't expose the outlet sensor.

**Alternative considered:** `exhaust_inlet − supply_inlet` (room-vs-outside Δ). Cleaner data (always available) but it's a property of the building, not the ventilation system. The proposal mentions "temperature gain", which the user phrased in heat-recovery terms.

### 5. Page placement

**Choice:** Insert the three new pages after `Page::TempIn` and before `Page::Status`. New `Page` enum:

```
Home, Bypass, TempIn, OutdoorHistory, IndoorHistory, DeltaHistory, Status, System, Settings
```

`PAGE_COUNT = 9`. The dots indicator already renders `PAGE_COUNT` dots evenly spaced — increasing the constant is the only change needed there. With 9 dots at 7 px pitch the row spans 56 px and remains centred (`(128 - 8·7)/2 = 36`).

**Rationale:** Group temperature-related pages together; the existing live `Intake` page acts as a natural "0 h" prelude to the history views.

### 6. Chart rendering

**Choice:** One vertical line per slot. Layout:

```
y=0..11   Header band (page title, drawn by draw_header)
y=12..23  Min / Max numeric strip ("Min 14.2 C   Max 21.7 C")
y=24..56  Chart area (33 px tall)
y=59..63  Dots indicator (existing)
```

For each slot `i` (0..127) that holds `Some(Bucket{min, max})`:
- Compute `y_top    = scale_y(max)` and `y_bottom = scale_y(min)`.
- Draw `Line::new(Point::new(i, y_top), Point::new(i, y_bottom))` with `BinaryColor::On`.
- If `min == max` (single sample bucket) draw a single pixel at `(i, y_top)`.

`scale_y` maps the channel-wide [min, max] into `y=24..56` inclusive, with a guard: if the range is < 0.5 °C wide, expand to ±0.25 °C around the midpoint to avoid a flat-line column collapsing to one row.

The newest slot (head − 1) renders at column 127 (rightmost) and older slots step left, wrapping at column 0 to (head modulo 128). I.e. the chart reads left-to-right as "24 h ago → now".

**Empty-state:** If every slot is `None` (just booted), draw the i18n `history_empty` string centred in the chart area instead of an empty box. The numeric Min / Max strip then renders `--` placeholders.

### 7. Dev-mode preload (added during implementation)

**Choice:** Under `feature = "simulate-ot"`, `TempHistory::new()` pre-fills all 128 slots of each channel with a synthetic 24 h summer-day cycle so the chart pages have something interesting on boot in dev mode.

**Pattern:**
- Outdoor: `20 + 8·cos(2π·(hour − 14)/24)` — peaks 28 °C at 14:00, dips to 12 °C at 02:00.
- Indoor: `22 + 0.5·sin(2π·(hour − 14)/24 + π/4)` — mild evening peak around 22.5 °C.
- Delta: `(indoor − outdoor) · 0.85` — models heat-exchanger gain (high at night when outdoor is cold, mildly negative mid-afternoon).
- Each slot is stored with `min = value − 0.3` and `max = value + 0.3` so the chart shows a visible band rather than a single pixel line.

Anchored at "now" = 14:00 so column 127 sits at the temperature peak; the chart reads left-to-right as "yesterday peak → night → today peak". The first real simulated sample (from `ot_master::simulate`) lands on the head slot and folds into the existing min/max, broadening it slightly — acceptable.

**Rationale:** Without this, the dev-mode chart shows `Collecting...` for the first 11 minutes of every boot — useless for visually iterating on the page layout. Compiled out entirely in release builds.

### 8. Dirty-flag integration

**Choice:** `TempHistory::sample()` returns a `bool` indicating "slot rolled" — i.e. the bucket index just advanced. Main loop ORs that into `display_dirty`.

**Rationale:** The chart only visibly changes when a slot rolls (~once every 11 min) or when the user navigates to a history page. Marking dirty every second would defeat the existing render-throttling. The min/max numeric strip lags by up to one bucket boundary, which is acceptable for a 24 h view.

When the user navigates to a history page, `next_page()` / `prev_page()` already calls `mark_dirty()`, so the first render after navigation happens immediately.

## Risks / Trade-offs

- **Cold-start blank chart:** For the first ~11 min after boot the chart shows only one column (current bucket). The empty-state hint covers the pre-first-sample window; partial fills are simply sparse. Acceptable — the alternative (synthesising fake history) is misleading.
- **No NTP dependency in bucketing:** `bucket_start_ms` uses `esp_timer_get_time()` (monotonic). That means buckets don't line up with wall-clock hours — "the last bucket is the most-recent ~11 min", not "10:00–10:11". The chart is a relative-time view; no time axis labels are drawn. Acceptable.
- **Delta page on unsupported hardware:** Wolf controllers that don't expose ID 81 leave the delta channel permanently empty. The page renders the empty-history hint, the same as during cold start. Documented in the proposal.
- **Memory:** ~3.5 KB on the heap (inside `AppStateInner`). Current main-loop stack already accommodates several KB of `AppStateInner`; adding 3.5 KB stays well under the ESP32 default task stack of 16 KB.
- **Float storage on a no-FPU temp range:** Temperatures are already `f32` in `cwl_data`. Storing as `f32` here keeps types consistent; an i16 fixed-point (0.1 °C resolution) would halve memory but adds conversion noise at every draw. Not worth optimising until profiling shows it.

## Open Questions

_None._ Sensor selection, persistence model, and sample resolution were confirmed during proposal Q&A.
