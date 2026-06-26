## 1. Tooltip infrastructure

- [x] 1.1 Add `hover` state (`{ x, y, content } | null`) to `TimelineChart` and make the card a positioned container (`position:relative`)
- [x] 1.2 Add a styled HTML tooltip `<div>` rendered from `hover` state, themed via existing CSS variables; add tooltip styles to `style.css`
- [x] 1.3 Add a helper to map a hovered SVG/DOM element to a card-relative `x,y` (via `getBoundingClientRect()` against the card rect) so positioning is correct under `viewBox` scaling
- [x] 1.4 Clamp the tooltip within the card width and flip below the point when near the top edge

## 2. Current-point highlight and value tooltip

- [x] 2.1 Compute the last non-null bucket for supply, exhaust, and (when shown) indoor/outdoor humidity
- [x] 2.2 Draw an emphasized current-point marker at each series' latest point using the existing `x`/`y`/`xH`/`yH` scales and `mid()`
- [x] 2.3 Wire pointer/focus handlers on the current-point markers (and the plot's right "now" edge) to show a tooltip with current supply/exhaust (and humidity) values plus the reading timestamp
- [x] 2.4 Dismiss the tooltip on mouse leave / blur

## 3. Marker and bypass hover tooltips

- [x] 3.1 Add an enlarged transparent hit area over each level-change marker; on hover/focus show a tooltip with the level and reason (reusing `tr.levels` / `tr.reasonLabels`, reboot handled)
- [x] 3.2 Add a full-height transparent hit overlay per bypass span; on hover/focus show a tooltip indicating bypass open (free cooling) vs closed (heat recovery)
- [x] 3.3 Remove the now-redundant native `<title>` tooltips for these elements (keep only if free)

## 4. Localization

- [x] 4.1 Add any missing tooltip strings (e.g. current-value/timestamp/"now" labels) to EN and DE in `translations.ts`, keeping both in sync
- [x] 4.2 Verify tooltip labels render correctly in German

## 5. Tests and verification

- [x] 5.1 Extend the timeline/StatusTab tests to assert the current-point marker is rendered and the value tooltip content is correct
- [x] 5.2 Add a test for marker and bypass hover tooltip content
- [x] 5.3 Run the web test suite and build; manually verify hover/tap behavior against the chart
