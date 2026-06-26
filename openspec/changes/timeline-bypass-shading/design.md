## Context

The temperature timeline already runs independently in the backend: `history.rs`
(`TempHistory`) folds supply/exhaust temperatures and indoor/outdoor humidity into rolling
ring buffers every second, persisted to MQTT under `persist/temp_history` separately from the
extreme-heat snapshot (`persist/extreme_heat`). The web chart (`ExtremeHeatChart.tsx`) is
rendered unconditionally in `StatusTab.tsx` and fetches `/api/history`, which embeds the
extreme-heat level-change events (`eh_events`) as its `events` array.

So the coupling is mostly *naming and framing*: the component, its events overlay, and the
mental model present the timeline as an extreme-heat artifact. Separately, there is **no
record of bypass state over time** — `requested_bypass_open` is a single live boolean set
from three places (extreme-heat in `extreme_heat.rs`, the calendar schedule in `scheduler.rs`,
and manual web/MQTT/encoder paths), with no history.

The bypass toggles only a handful of times per day, so a small transition log is sufficient to
reconstruct its state across the 24 h window — the same shape as the existing `eh_events`
ring buffer.

## Goals / Non-Goals

**Goals:**
- Reframe the chart as a standalone temperature timeline that does not depend on Extreme Heat
  mode; extreme-heat level changes remain an optional overlay.
- Record bypass open/closed transitions across the history window and persist them with the
  history snapshot.
- Render the chart background in two gray tones — base gray for bypass-closed spans, a
  slightly brighter gray for bypass-open spans — aligned to the time axis, behind the curves.

**Non-Goals:**
- No change to *how* the bypass is decided (that is the extreme-heat-bypass-coupling change).
- No new HTTP endpoint and no config/NVS schema change.
- No per-second bypass sampling into a min/max bucket — bypass is a sparse boolean state, so a
  transition log is the right model, not a `Channel`.
- No change to the extreme-heat level-change marker semantics.

## Decisions

### Model bypass history as a transition log, not a history Channel
Add a bounded ring buffer of `{epoch, open}` transitions to `AppStateInner` (e.g.
`bypass_events: VecDeque<BypassEvent>`, capped like `EH_EVENT_CAPACITY`). A transition is
appended only when `requested_bypass_open` actually changes. Rationale: bypass is a piecewise-
constant boolean that changes a few times a day; a transition log is far cheaper than a
1440-slot channel and reconstructs exact band edges, whereas per-bucket booleans would blur
edges and bloat the payload. Alternative considered: a boolean `Channel` per bucket — rejected
(memory + edge precision). Recording happens at a single chokepoint: a small helper that all
three writers (extreme-heat, scheduler, manual/MQTT) funnel `requested_bypass_open` through,
so no writer can change the damper without logging it.

### Reconstruct the left-edge state from the earliest known transition
To paint from the oldest visible column, the frontend needs the bypass state at the window's
left edge. The state in the span *before* the earliest transition in the buffer is the inverse
of that transition's `open` value (a transition flips the state). With the cap sized to cover
24 h of realistic toggling this is exact; if the buffer ever overflows, the worst case is one
mislabeled leading span, which is acceptable for a background tint.

### Expose via the existing `/api/history` payload
`TempHistory::full_json` already takes a pre-serialized `events` string; add a parallel
`bypass` array of `{epoch, open}` built in `webserver.rs` from `bypass_events` (same way
`events` is built from `eh_events`). Keeps one fetch and one render pass. Persist the
transitions inside the retained `persist/temp_history` snapshot (not the extreme-heat one), so
the shading survives reboots together with the curves it annotates.

### Render as background `<rect>` bands behind everything
In the chart, before drawing gridlines/series, emit one `<rect>` per bypass span spanning
`PAD_T..PAD_T+PLOT_H`, x-mapped with the existing `eventX` epoch→x helper, filled with a base
gray (closed) or a slightly brighter gray (open). Two CSS custom properties
(e.g. `--chart-bg`, `--chart-bg-bypass`) keep the tones theme-aware and the difference subtle.
Add a legend swatch + EN/DE strings explaining "bypass open / closed".

### Rename the component, keep extreme-heat markers as an overlay
Rename `ExtremeHeatChart.tsx` → `TimelineChart.tsx` (and the export) and update the import in
`StatusTab.tsx`. The level-change markers and their legend stay, but are now clearly an
overlay on a general-purpose timeline. No behavioural change to the markers.

## Risks / Trade-offs

- **Buffer overflow on a pathological day (many toggles)** → the cap covers far more than the
  realistic few-per-day; on overflow only the oldest leading span tint may be wrong, never the
  curves. Mitigation: size the cap generously and drop oldest first.
- **Subtle two-gray difference may be invisible in some themes** → expose both tones as CSS
  variables per theme and pair the shading with a legend entry so it is discoverable; keep the
  brighter tone clearly distinguishable but not loud (it is context, not a series).
- **Pre-existing retained snapshots lack the bypass list** → treat a missing `bypass` field as
  "no shading data yet"; the chart renders exactly as today until new transitions accrue.
