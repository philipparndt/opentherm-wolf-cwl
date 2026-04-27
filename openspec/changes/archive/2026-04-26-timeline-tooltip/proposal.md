## Why

The weekly timeline segments currently show a basic browser `title` tooltip with just the level name and time range. Users want a richer tooltip that also shows the air volume exchanged during that segment, making it easy to understand the contribution of each schedule block at a glance.

## What Changes

- Replace the native `title` attribute tooltip with a custom styled tooltip
- Show: time range (HH:MM - HH:MM), level name, duration, and volume (m³) for that segment
- For gap segments: show "No program" with the uncovered time range
- Tooltip follows the mouse cursor and stays within the viewport

## Capabilities

### New Capabilities

(none — web UI enhancement)

### Modified Capabilities

(none)

## Impact

- **Modified files**: Web UI `App.tsx` (`WeekTimeline` component), `style.css` (tooltip styles)
- No firmware changes
