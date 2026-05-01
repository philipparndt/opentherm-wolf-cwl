## Why

The schedule timeline in the web UI has two usability gaps:
1. When dragging a schedule segment's start/end handle, there is no indicator showing the exact time being dragged to. Users have to guess position based on the axis ticks (0, 6, 12, 18, 24) which is imprecise.
2. The weekly overview timeline is completely hidden when no schedule entries exist (`entries.length === 0`). This means new users don't see the visual timeline until after they've added entries, making the feature less discoverable and removing the context of "current time" orientation.

## What Changes

- Show a time label (e.g. "14:30") near the cursor while dragging a schedule segment handle, updating in real-time as the user moves the mouse
- Always display the WeekTimeline component regardless of whether entries exist, so the time-of-day axis and current-time indicator are visible even with an empty schedule

## Capabilities

### New Capabilities
- `schedule-drag-time-label`: Display a floating time indicator during drag operations on schedule handles
- `schedule-empty-timeline`: Show the WeekTimeline overview even when no schedule entries exist

### Modified Capabilities

## Impact

- `esp32/web/src/App.tsx` — WeekTimeline component: add drag time indicator, remove conditional hiding
- `esp32/web/src/style.css` — Styling for the drag time label
