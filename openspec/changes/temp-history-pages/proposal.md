## Why

The OLED display currently shows only instantaneous temperatures (the `Intake` page reports the live supply- and exhaust-inlet readings). There is no way to see how outdoor / indoor air or heat-recovery gain has trended over time — a typical question after a cold night ("did the bypass open too early?", "how cold did it get?") cannot be answered from the panel. MQTT subscribers can graph this externally, but the device itself goes blind the moment you walk up to it.

A small in-RAM ring buffer is enough to give a 24 h glance directly on the panel without any flash wear, NTP dependency, or schema migration.

## What Changes

- New in-memory temperature history module (`src/history.rs`) — three ring buffers, 128 buckets of ~11.25 min each (24 h total), for outdoor / indoor / delta. Each bucket stores aggregated min and max. RAM-only; reset on reboot.
- Sampler integrated into the main loop: once per second the latest `cwl_data` temperatures are folded into the current bucket; when the bucket window elapses, head advances.
- Three new display pages:
  - `OutdoorHistory` — `supply_inlet_temp` 24 h trend.
  - `IndoorHistory` — `exhaust_inlet_temp` 24 h trend.
  - `DeltaHistory` — `supply_outlet_temp − supply_inlet_temp` (heat-recovery gain) 24 h trend; falls back to a "—" placeholder until ID 81 is observed as supported.
- Each page shows the header (page title + units), the 24 h min and max in the top band, and a 128-px-wide auto-scaled chart showing the per-bucket [min, max] range as a vertical bar per column.
- `PAGE_COUNT` grows from 6 to 9. New pages sit after `TempIn` and before `Status` so they are grouped with the live temperature view.
- i18n strings added in `i18n.rs` (EN + DE) for the three new headers, "Min", "Max", and the empty-history hint.

## Capabilities

### New Capabilities

- 24 h on-device temperature history with min/max + chart for outdoor air, indoor air, and heat-recovery gain.

### Modified Capabilities

- Display page navigation now cycles through 9 pages instead of 6.

## Impact

- **`src/history.rs`** (new): ring buffer, bucket aggregation, sampler API.
- **`src/cwl_data.rs`**: no changes (history is a separate concern; it reads from `CwlData` via the sampler).
- **`src/app_state.rs`**: add `temp_history: history::TempHistory` field; constructor wires it up.
- **`src/main.rs`**: declare `mod history;` and call `state.temp_history.sample(...)` once per second in the main loop; mark display dirty when a new bucket boundary is crossed so the chart redraws.
- **`src/display.rs`**: extend `Page` enum with `OutdoorHistory`, `IndoorHistory`, `DeltaHistory`; raise `PAGE_COUNT` to 9; add `draw_outdoor_history`, `draw_indoor_history`, `draw_delta_history` and a shared `draw_temp_chart` helper; route in `render_content`.
- **`src/i18n.rs`**: new fields `outdoor_24h`, `indoor_24h`, `delta_24h`, `min_label`, `max_label`, `history_empty` on `Strings`, populated for `EN` and `DE`.
- **No flash, NVS, MQTT, or web-API changes.**
