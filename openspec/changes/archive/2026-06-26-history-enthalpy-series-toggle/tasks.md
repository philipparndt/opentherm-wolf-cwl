## 1. Firmware — record enthalpy history

- [x] 1.1 Add `indoor_enthalpy` and `outdoor_enthalpy` `Channel`s to `TempHistory` in
      `esp32/src/history.rs`, constructed with `Channel::with_len(HUMIDITY_SLOTS)` in `new()`.
- [x] 1.2 Extend `sample_humidity` to accept `indoor_h: Option<f32>` and `outdoor_h: Option<f32>`,
      advancing the enthalpy channels on the same bucket boundary and folding each `Some` value.
- [x] 1.3 In `esp32/src/main.rs`, call `humidity::inputs(st, now_ms, exhaust_temp, supply_temp)`
      in the history-sampling block and pass `indoor.h` / `outdoor.h` (`None` when absent) into
      `sample_humidity`.
- [x] 1.4 (If `simulate-ot`) optionally preload synthetic enthalpy — intentionally skipped to
      stay consistent with humidity, which is likewise not preloaded in the simulation.

## 2. Firmware — serialize enthalpy to the web

- [x] 2.1 In `full_json`, emit `indoorEnthalpy` and `outdoorEnthalpy` arrays via
      `append_json_array`, and grow the pre-sized `String::with_capacity` estimate accordingly.

## 3. Firmware — persist enthalpy in the MQTT snapshot

- [x] 3.1 In `snapshot_json`, add an `enthPoints` list (downsampled like `humPoints`, reusing
      `MQTT_HUMIDITY_POINTS`), emitting points only where a channel has data, and enlarge the
      reserved buffer capacity.
- [x] 3.2 In `restore_from_snapshot`, add a parse branch for `enthPoints` that re-bins by epoch
      onto the 5-minute grid, forward-fills gaps, and bounds the scan by the next section key so
      enthalpy/humidity/bypass points aren't cross-ingested.
- [x] 3.3 Extend the snapshot round-trip unit test to cover enthalpy, and add a test that a
      snapshot without `enthPoints` still restores temperature/humidity (backward compatibility).

## 4. Web — history shape and enthalpy curves

- [x] 4.1 Add optional `indoorEnthalpy?` / `outdoorEnthalpy?` arrays to the `History` interface
      in `esp32/web/src/api.ts`.
- [x] 4.2 In `TimelineChart.tsx`, build enthalpy segments on their own auto-scaled kJ/kg axis,
      drawn in a distinct style/colour, with current-point markers and a value-tooltip line.
- [x] 4.3 Add the enthalpy axis labels/legend entries, and the unit label, only when enthalpy
      data is present.

## 5. Web — series/overlay visibility toggles

- [x] 5.1 Add a visibility state record (curves + level-change markers + bypass strip), default
      all-on, loaded from and saved to `localStorage`, read defensively (unknown → visible).
- [x] 5.2 Make each legend entry a toggle control (clickable + keyboard-focusable) with a clear
      on/off affordance (e.g. dimmed swatch when off).
- [x] 5.3 Skip hidden curves in the plot, the current-point markers, and the value tooltip; show
      each value axis only when at least one of its visible series remains; rescale axes to the
      visible series.
- [x] 5.4 Add toggle/axis/legend labels to `esp32/web/src/translations.ts` (EN + DE) and any
      needed styles to `esp32/web/src/style.css`.

## 6. Verify

- [x] 6.1 `cargo build` (esp32) and the web build/lint pass; firmware history unit tests pass.
- [x] 6.2 Sanity-check RAM/heap headroom and that the MQTT snapshot still fits its buffer with
      the added enthalpy block.
- [x] 6.3 Manually verify on the live controller (10.10.2.60): enthalpy curves render, toggles
      hide/show series and persist across reload, and history survives a reboot via the snapshot.
