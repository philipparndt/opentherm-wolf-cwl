## Why

Creating and editing schedules currently requires opening a modal dialog, typing time ranges, and selecting days — multiple clicks per schedule. An interactive timeline where users can click to add segments and drag borders to adjust times would be much faster and more intuitive, especially for visually planning the week.

## What Changes

- Make the weekly overview timeline interactive (editable)
- Click on a gap to add a new schedule segment at that position
- Drag the borders between segments to adjust start/end times
- Click on a segment to change its level (cycles through Off/Reduced/Normal/Party)
- Changes apply to the underlying schedule entries and are saved to the device
- Keep the existing modal dialog as an alternative for precise editing
- Snap times to 15-minute increments for usable drag resolution

## Capabilities

### New Capabilities

(none — enhancement to existing schedule UI)

### Modified Capabilities

(none)

## Impact

- **Modified files**: Web UI `App.tsx` (interactive `WeekTimeline` with drag/click handlers, schedule mutation logic), `style.css` (drag handles, cursor styles)
- No firmware changes — the schedule data model and API remain the same
