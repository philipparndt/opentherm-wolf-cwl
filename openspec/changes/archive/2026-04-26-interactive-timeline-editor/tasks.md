## 1. Segment source tracking

- [x] 1.1 Extend `computeDaySegments` to return the source entry index per segment

## 2. Drag borders

- [x] 2.1 Add drag handles at segment boundaries — thin hit areas on the left/right edges
- [x] 2.2 Implement drag logic — mousedown/mousemove/mouseup, snap to 15-min, update entry start/end
- [x] 2.3 Add drag CSS — cursor:col-resize handles, hover highlight

## 3. Click interactions

- [x] 3.1 Click on gap → add new segment — 2h Normal block centered on click, weekdays/weekend auto-detected
- [x] 3.2 Click on segment → cycle level — Off→Reduced→Normal→Party→Off

## 4. Integration

- [x] 4.1 Pass `onEntriesChange` callback from `SchedulesTab` to `WeekTimeline`
- [x] 4.2 Build verification — SUCCESS (32.6KB JS)
