## Why

The weekly timeline and daily volume table are currently separate cards, and the container is limited to 600px max-width which makes the timeline bars too narrow to read comfortably. By merging the volume into the timeline rows and widening the content area, the overview becomes a single, information-dense component that's easy to scan.

## What Changes

- Merge the `DailyVolume` table into the `WeekTimeline` — each day row shows the timeline bar plus the total volume on the right
- Remove the separate `DailyVolume` card
- Increase the `.container` max-width to give the timeline more horizontal space
- The weekly overview card uses the full available width

## Capabilities

### New Capabilities

(none — UI layout improvement)

### Modified Capabilities

(none)

## Impact

- **Modified files**: Web UI `App.tsx` (merge DailyVolume into WeekTimeline, remove separate component), `style.css` (wider container, adjusted timeline layout)
- No firmware changes
