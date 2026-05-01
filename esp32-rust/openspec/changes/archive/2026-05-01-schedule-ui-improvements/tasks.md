## 1. Add drag time indicator

- [x] 1.1 Add `dragTime` state to WeekTimeline: `useState<{ x: number; y: number; text: string } | null>(null)`
- [x] 1.2 In `handleDragStart` onMove handler, compute the snapped time as HH:MM string and update `dragTime` state with cursor position and text
- [x] 1.3 In `handleDragStart` onUp handler, clear `dragTime` to null
- [x] 1.4 Render the drag time label as a fixed-position element when `dragTime` is not null (similar pattern to existing tooltip)
- [x] 1.5 Add CSS styling for `.tl-drag-time` label: small font, background, rounded, `pointer-events: none`, positioned above cursor

## 2. Always show WeekTimeline

- [x] 2.1 Remove the `entries.length > 0 &&` condition before `<WeekTimeline>` — render it unconditionally
- [x] 2.2 Remove the `entries.length === 0 && <div>No schedules configured</div>` fallback block

## 3. Verify

- [x] 3.1 Run `cd esp32/web && npm run build` to confirm the web UI builds successfully
