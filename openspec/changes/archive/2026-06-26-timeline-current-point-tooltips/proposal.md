## Why

The temperature timeline renders supply/exhaust (and humidity) curves, level-change markers, and
the bypass state strip, but it is read-only at a glance: the **latest reading** isn't called out,
so you can't see the current value without cross-referencing the status card, and the only context
on hover is the browser's native SVG `<title>` tooltip — slow to appear, unstyled, and absent for
the curves themselves. Users want to point at the chart and immediately read *what the value is
now* and *why the bypass/level changed*.

## What Changes

- **Highlight the current data point.** Draw an emphasized marker (dot) at the most recent
  populated bucket of each temperature curve (and humidity, when shown), so the "now" end of each
  line is visually anchored.
- **Hover tooltip for the current value.** Hovering the highlighted current point (or the plot's
  right edge) shows a styled tooltip with the current supply/exhaust temperature (and humidity)
  and its timestamp — replacing reliance on native `<title>` for the headline value.
- **Hover tooltips that explain the markers.** Hovering a level-change marker shows the level and
  the reason it changed (temp, cooling, dehumidify, muggy, manual, schedule, reboot) in a styled
  tooltip rather than the native title.
- **Hover tooltip on the bypass strip.** Hovering a bypass span shows whether the bypass was open
  (free cooling) or closed (heat recovery) over that span, in the same styled tooltip.
- The tooltips are a frontend interaction layer; no firmware, API, or data-model change.

## Capabilities

### New Capabilities
- `timeline-interaction`: hover-driven tooltips and current-point highlighting on the temperature
  timeline — emphasizing the latest reading and surfacing the reason behind level-change markers
  and bypass spans.

### Modified Capabilities
<!-- The temperature-timeline and bypass-timeline-shading capabilities (from the
     timeline-bypass-shading change) keep their existing rendering requirements; this change only
     adds an interaction layer on top, so no existing requirement is altered. -->

## Impact

- Web frontend (`esp32/web/src/TimelineChart.tsx`): add a current-point marker per series, an
  interaction layer (pointer handlers / overlay) driving a styled HTML tooltip, and hover hit
  targets for level-change markers and bypass spans. Replaces native `<title>` tooltips for these
  elements.
- Translations (`esp32/web/src/translations.ts`): EN/DE strings for tooltip labels (current value,
  bypass open/closed, timestamp/"now"). Reuses existing level/reason labels.
- Tests (`esp32/web/src/`): extend the timeline/StatusTab tests for the current-point marker and
  tooltip content.
- No change to `/api/history`, firmware, or persisted data — the timestamp and values are already
  in the existing history payload.
