## Why

The status page already computes and shows live specific enthalpy (kJ/kg) for indoor and
outdoor air, but it is not retained in history, so trends over the day are invisible. Enthalpy
is the metric that actually drives the extreme-heat / free-cooling decisions, so seeing its
history would make those decisions legible. The chart is also getting crowded (supply, exhaust,
two humidity curves, level-change markers, bypass shading), so adding more series without a way
to declutter would hurt readability.

## What Changes

- Record indoor and outdoor **specific enthalpy (kJ/kg)** into the history ring buffer at
  5-minute resolution (the same cadence and slot count already used for humidity), reusing the
  already-computed `AirState.h` value — no new psychrometric work.
- Serialize the two enthalpy channels in the `/api/history` payload and round-trip them through
  the MQTT history snapshot (persist + restore) like the existing channels.
- Render the two enthalpy curves on the timeline (on the humidity-style secondary axis).
- Add **per-series / per-overlay visibility toggles** to the timeline so the user can turn
  individual curves (supply, exhaust, indoor/outdoor humidity, indoor/outdoor enthalpy) and the
  marker/shading overlays on and off to keep the chart readable.

## Capabilities

### New Capabilities
- `timeline-series-toggle`: User-controlled show/hide of individual timeline curves and overlay
  layers, with the selection persisted across reloads.

### Modified Capabilities
- `temperature-timeline`: Adds indoor and outdoor enthalpy as history-backed curves rendered on
  the timeline at 5-minute resolution.
- `mqtt-state-recovery`: The history snapshot adds the indoor/outdoor enthalpy channels so they
  survive a reboot like the temperature and humidity channels.

## Impact

- Firmware (Rust): `esp32/src/history.rs` (new enthalpy channels, JSON emit, snapshot
  persist/restore), `esp32/src/main.rs` (fold `AirState.h` on the humidity sampling tick),
  `esp32/src/humidity.rs` (already exposes enthalpy — read only).
- Web (Preact/TS): `esp32/web/src/api.ts` (new optional history arrays), `TimelineChart.tsx`
  (enthalpy series + toggle UI/state), `style.css`, `translations.ts` (toggle labels, EN/DE).
- RAM: ~6.8 KB additional heap (2 channels × 288 slots × 12 bytes) on the no-PSRAM ESP32 —
  negligible against the 520 KB internal budget.
- MQTT snapshot payload grows by two more channel arrays.
