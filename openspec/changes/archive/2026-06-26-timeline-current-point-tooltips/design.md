## Context

`TimelineChart.tsx` renders a single inline SVG (a `viewBox`-scaled 640×232 coordinate system)
containing: gridlines, the supply/exhaust temperature series, an optional humidity overlay on a
right-hand axis, vertical level-change markers, and a bypass state strip below the plot. Series are
drawn from runs of non-`null` buckets via the `segments()` helper; markers and the bypass strip
already carry native SVG `<title>` children for context.

The chart fetches `/api/history` every 30 s. The payload already contains everything the tooltips
need: `nowEpoch`, `bucketMs`, the per-bucket `[min,max]` arrays, the `events` (level changes), and
the `bypass` transition log. So this is a **frontend-only interaction layer** — no firmware or API
change.

Constraints: the codebase is Preact (`preact/hooks`), the chart is a single function component with
no existing interaction state, and the SVG is scaled to 100 % width via `viewBox` (so screen pixels
≠ SVG units). Tooltips must work under that scaling and on a touch screen.

## Goals / Non-Goals

**Goals:**
- Emphasize the latest populated point of each series with a current-point marker.
- Show a styled (HTML, theme-aware) tooltip on hover for: the current value, level-change markers,
  and bypass spans — replacing the native `<title>` for these elements.
- Keep the tooltip positioned correctly despite the `viewBox` scaling, and localized (EN/DE).

**Non-Goals:**
- No crosshair / scrubbing tooltip that follows the pointer across the whole time axis to read any
  historical bucket — only the current point gets a value tooltip (markers/bypass get reason
  tooltips). A full scrubber can be a later change.
- No `/api/history`, firmware, or persisted-data change.
- No new charting dependency.

## Decisions

### Styled HTML tooltip overlay, not native `<title>`
Render a single absolutely-positioned HTML tooltip `<div>` inside the chart card (which becomes
`position:relative`), driven by component state `hover: { x, y, content } | null`. SVG elements get
`onMouseEnter`/`onMouseLeave` (and `onFocus`/`onBlur` for keyboard/touch) handlers that set the
hover state with the element's position and the text to show. Rationale: native `<title>` is
unstyled, slow to appear, and can't show multi-line rich content or match the theme; an HTML
overlay is fully controllable and reuses existing CSS variables. Alternative considered: a foreign
`<foreignObject>` tooltip inside the SVG — rejected as more awkward to position/scale than an HTML
sibling. Keep the native `<title>` only as a no-JS fallback if cheap, otherwise remove it.

### Position via SVG→screen coordinate mapping
Because the SVG is `viewBox`-scaled, convert SVG units to pixels using the rendered element's
`getBoundingClientRect()` / `getScreenCTM()` (or anchor the tooltip off the hovered DOM element's
own client rect) rather than assuming 1 SVG unit = 1 px. Store the resulting card-relative `x,y` in
hover state and offset the tooltip above the point, clamping within the card width so it isn't
clipped at the edges.

### Current-point marker derived from the last non-null bucket
For each series, find the last index `i` with a non-`null` bucket and draw an emphasized marker
(larger filled dot with a contrasting ring) at `(x(i), y(mid(bucket)))` — reusing the existing `x`,
`y`, `xH`, `yH` scales and `mid()` helper. This is the same point the curve already ends at, so the
marker simply annotates it. The current-value tooltip reads those last buckets plus `nowEpoch` for
the timestamp.

### Enlarge hit targets for thin elements
Level-change markers are 1px vertical lines and bypass spans are 7px tall — hard to hover. Give each
an invisible wider hit area (a transparent `<rect>` over the marker line / a full-height transparent
overlay per bypass span) carrying the pointer handlers, so hovering is forgiving without changing
the visible rendering.

### Reuse existing translation maps; add only tooltip-specific strings
Level names (`tr.levels`), reason labels (`tr.reasonLabels`), and bypass open/closed
(`tr.bypassOpen`/`tr.bypassClosed`) already exist. Add only what's missing for the current-value
tooltip (e.g. labels for supply/exhaust/humidity already exist as `tr.supply` etc.; add a "now" /
timestamp label if needed). Keep EN and DE in sync.

## Risks / Trade-offs

- **Coordinate mapping under `viewBox` scaling is fiddly** → anchor the tooltip off each hovered
  DOM element's `getBoundingClientRect()` relative to the card's rect, which is resolution- and
  scale-independent, rather than hand-computing from SVG units.
- **Touch devices have no hover** → wire `onFocus`/`onClick`/`onTouchStart` alongside hover so a tap
  shows the tooltip; dismiss on the next tap elsewhere. Acceptable if tap-to-show is best-effort.
- **Tooltip clipped at card edges** → clamp the tooltip's left position within the card and flip it
  below the point when near the top edge.
- **Removing native `<title>` loses the no-JS tooltip** → JS is required for the SPA anyway, so the
  loss is immaterial; keep a `<title>` only if it costs nothing.
