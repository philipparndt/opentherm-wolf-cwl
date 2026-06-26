## Context

The timeline (`esp32/web/src/TimelineChart.tsx`) already draws supply/exhaust temperature
(left axis) and indoor/outdoor humidity (right 0–100 % axis), plus level-change markers and a
bypass state strip. The firmware keeps these in RAM-only ring buffers in `esp32/src/history.rs`:
two temperature `Channel`s at 1-minute resolution (1440 slots) and two humidity `Channel`s at
5-minute resolution (288 slots), each slot an `Option<Bucket{min,max:f32}>` (~12 bytes). The
buffers round-trip through a retained MQTT snapshot (`snapshot_json` / `restore_from_snapshot`)
so history survives reboots.

Specific enthalpy is already computed live: `humidity::inputs()` returns `HumidityInputs` with
indoor/outdoor `AirState`, and `AirState.h` is the specific enthalpy in kJ/kg. It is shown on
the status page but never stored. The chip is a plain ESP32 with **no PSRAM** (520 KB internal
RAM); the current history buffer is ~41.5 KB.

## Goals / Non-Goals

**Goals:**
- Retain indoor and outdoor specific enthalpy (kJ/kg) as history-backed timeline curves.
- Use 5-minute resolution (the humidity cadence) to keep the added RAM negligible (~6.8 KB).
- Reuse the already-computed `AirState.h` — no new psychrometric math.
- Persist enthalpy across reboots via the existing MQTT snapshot.
- Let the user show/hide individual curves and overlay layers so the busier chart stays legible,
  remembering the choice across reloads.

**Non-Goals:**
- No change to the extreme-heat decision logic (enthalpy already drives it).
- No change to temperature/humidity resolution or the OLED rendering.
- No server-side persistence of the toggle state (it is a per-browser display preference).
- No new sensors or MQTT topics beyond the enlarged snapshot payload.

## Decisions

### 1. Store enthalpy as two coarse channels alongside humidity
Add `indoor_enthalpy` and `outdoor_enthalpy` `Channel`s to `TempHistory`, each
`Channel::with_len(HUMIDITY_SLOTS)` (288 slots, 5-min buckets). They share the humidity
bucket clock (`humidity_bucket_start_ms`) and advance in the same `sample_humidity` tick, so no
new timer state is needed.

*Why:* Enthalpy is derived from the same slow-moving temp/RH inputs as humidity, so the 5-min
grid is appropriate and the per-channel `min/max` Bucket type fits unchanged. Reusing the
humidity clock keeps the columns aligned for rendering.

*Alternative considered:* 1-minute resolution (1440 slots, ~34.5 KB) — rejected; enthalpy moves
no faster than humidity, so the extra RAM and payload buys nothing.

### 2. Feed enthalpy from `humidity::inputs()` at the sample site
Rename/extend `sample_humidity` to also accept `indoor_h: Option<f32>` and
`outdoor_h: Option<f32>`. In `main.rs`, where the humidity sample is taken, call
`humidity::inputs(st, now_ms, exhaust_temp, supply_temp)` and pass `indoor.h` / `outdoor.h`
(both `None` when that side has no fresh sensor, exactly like the RH path).

*Why:* `inputs()` already performs the lowest-temp / highest-RH aggregation and pressure
handling; the enthalpy it returns is the same value shown on the status page, keeping the chart
consistent with the live reading. `None` folds nothing, matching humidity behaviour.

*Alternative considered:* recompute enthalpy from the aggregated RH inside `history.rs` —
rejected; it would duplicate the aggregation and could diverge from the displayed value.

### 3. Extend the snapshot, keeping it backward-compatible
Add an `enthPoints` list to `snapshot_json` (epoch-stamped, downsampled like `humPoints`) and a
matching parse branch in `restore_from_snapshot`. The parser must bound each section's scan by
the start of the next key, as the existing code already does for `humPoints` vs `bypass`.
Snapshots without `enthPoints` (older firmware) restore as today — the field is simply absent.

*Why:* Mirrors the proven humidity persistence path; forward/backward compatible because every
section is located by key, not position.

*Risk:* Payload size — see Risks.

### 4. Enthalpy gets its own auto-scaled value axis
Temperature owns the left axis (°C); humidity owns the right axis (0–100 %). Enthalpy (kJ/kg,
typically ~20–90 in summer) does not share either domain. Draw it against its own auto-scaled
range (min/max across both enthalpy series, padded), rendered in a distinct style (e.g. dotted)
and colour, with a compact axis/legend label noting the unit. Because the user can toggle series
off, axis clutter is mitigated by the toggle feature itself.

*Alternative considered:* reuse the right 0–100 axis — rejected; enthalpy and RH would overlap on
an axis whose label (%) is wrong for one of them, which is misleading.

### 5. Toggles as clickable legend entries, persisted in localStorage
Make each legend swatch a toggle button controlling the visibility of its curve/overlay. Keep a
small visibility record (`{supply, exhaust, indoorHum, outdoorHum, indoorEnth, outdoorEnth,
markers, bypass}`), default all-on, persisted to `localStorage` and read on mount. Hidden series
are skipped in both the plot and the current-point/value tooltip; axes appear only when at least
one series using them is visible.

*Why:* The legend already enumerates every series with a swatch; making those swatches the
control is discoverable and adds no separate UI panel. localStorage keeps it a pure client
preference with no firmware/API involvement.

*Alternative considered:* a separate checkbox panel or query-param state — rejected as heavier
than reusing the existing legend; localStorage is the lightest durable per-browser store.

## Risks / Trade-offs

- **MQTT snapshot grows by one more point list** → enthalpy reuses the humidity downsample
  count (`MQTT_HUMIDITY_POINTS`, 100 points) and only emits points where a channel has data, so
  the increase is bounded and similar to the existing humidity block. Verify the combined
  payload still fits the MQTT buffer; reduce the enthalpy point count if needed.
- **RAM headroom on a no-PSRAM ESP32** → +~6.8 KB heap against a ~41.5 KB baseline and 520 KB
  total; negligible, but the snapshot-build transient string also grows slightly. Pre-size the
  snapshot `String` capacity to include the new block.
- **Enthalpy axis crowds the plot** → mitigated by the new visibility toggles; default could
  ship with enthalpy hidden if the chart feels busy (decided in implementation/review).
- **Legend-as-toggle discoverability** → add a subtle affordance (cursor/hover, dimmed swatch
  when off) and ensure keyboard focus + activation work, consistent with existing `chart-hit`
  accessibility.
- **Stale localStorage shape** → read defensively (unknown keys default to visible) so a future
  series addition doesn't hide everything.

## Migration Plan

1. Ship firmware: new channels are empty on boot and fill within 5 minutes; older retained
   snapshots restore without enthalpy (absent field) — no migration step.
2. Ship web: defaults to all series visible (or enthalpy-off if chosen); localStorage is created
   on first toggle.
3. Rollback: reverting the web bundle drops the toggles and enthalpy curves; reverting firmware
   stops emitting `enthPoints` (harmlessly ignored by older/newer parsers). No data migration to
   undo since history is RAM-only + retained snapshot.

## Open Questions

- Default visibility for the enthalpy curves on first load — on, or off to keep the initial view
  calm? (Lean: ship on; revisit in review.)
- Exact enthalpy line style/colour to stay distinct from the dashed humidity curves (dotted vs.
  a new hue) — a visual-polish call for implementation.
