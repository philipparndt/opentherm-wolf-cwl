## Context

The `WeekTimeline` component (App.tsx lines 104-324) renders a horizontal bar chart per day with drag handles for adjusting schedule start/end times. Dragging calls `xToMinutes()` to convert mouse position to 15-minute-snapped time values, but no visual feedback of the exact time is shown to the user.

The timeline is conditionally rendered at line 440: `{entries.length > 0 && <WeekTimeline ... />}`. When empty, a "No schedules configured" message is shown instead.

## Goals / Non-Goals

**Goals:**
- Show a floating time label (HH:MM format) tracking the cursor during drag
- Always render the WeekTimeline regardless of entry count
- Keep the "click gaps to add" interaction working on the empty timeline

**Non-Goals:**
- Changing drag snapping behavior (stays at 15-minute intervals)
- Adding new schedule entry creation UI beyond existing gap-click
- Changing the time axis tick marks or labels

## Decisions

### 1. Drag time indicator as tooltip-style floating element

**Choice**: Add a `dragTime` state (`{ x: number; y: number; text: string } | null`) that is set during drag moves and cleared on drag end. Render as a small absolutely-positioned label near the cursor (offset slightly above/to the right to avoid obscuring the handle).

**Rationale**: Follows the same pattern as the existing `tip` tooltip state. Minimal code addition. The label shows the snapped time (what will actually be applied), not the raw pixel position.

**Alternative considered**: Showing time inside the segment itself — rejected because narrow segments have no space for text.

### 2. Time label format and positioning

**Choice**: Show time as "HH:MM" (e.g. "14:30") in a small label positioned 20px above the cursor. Use `position: fixed` with `pointer-events: none` (same approach as existing tooltip).

**Rationale**: Fixed positioning relative to cursor is simplest and works regardless of scroll position. The format matches the axis labels and schedule list entries.

### 3. Always render WeekTimeline

**Choice**: Remove the `entries.length > 0` condition. Pass entries (empty array) to WeekTimeline unconditionally. The component already handles empty segments via `computeDaySegments` which will show full-width gaps.

**Rationale**: The timeline with its time axis, current-time indicator, and gap-click-to-add behavior is useful even when empty. It provides orientation and makes the "click to add" feature discoverable.

### 4. Remove "No schedules configured" fallback message

**Choice**: Remove the `entries.length === 0 && <div>No schedules configured</div>` block. The empty timeline with clickable gaps replaces it — the hint "click gaps to add" in the header already explains the interaction.

## Risks / Trade-offs

- **[Empty timeline looks sparse]** → The 7 rows with day labels, time axis, and "now" line still provide useful orientation. Gap segments are clickable for adding new schedules.
- **[Drag label flicker]** → Using `requestAnimationFrame` or React state batching prevents this; the existing tooltip already works without flicker at the same update rate.
