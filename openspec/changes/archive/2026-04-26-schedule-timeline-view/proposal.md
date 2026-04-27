## Why

The grouped schedule list shows times and levels, but it's hard to see the overall weekly picture — are there gaps where no schedule is active? Do schedules overlap? What happens at 3am on a Tuesday? A visual timeline for each day of the week makes coverage immediately obvious and highlights uncovered time zones.

## What Changes

- Add a weekly timeline visualization above the schedule list in the Schedules tab
- One horizontal bar per day (Mon-Sun), spanning 0:00-24:00
- Color-coded segments for each ventilation level (Off, Reduced, Normal, Party)
- Uncovered time zones shown in a distinct warning color (striped/hatched pattern or dimmed red)
- Time axis labels (0, 6, 12, 18, 24) for orientation
- Midnight-spanning schedules render correctly (e.g., 22:00-06:00 wraps)

## Capabilities

### New Capabilities

(none — web UI display enhancement only)

### Modified Capabilities

(none)

## Impact

- **Modified files**: Web UI `App.tsx` (new `WeekTimeline` component), `style.css` (timeline styles)
- No firmware changes
