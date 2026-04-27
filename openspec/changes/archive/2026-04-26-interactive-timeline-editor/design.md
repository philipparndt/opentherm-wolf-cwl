## Context

The `WeekTimeline` component currently renders read-only segments from the `entries` schedule array. The `SchedulesTab` manages `entries` state and saves via `saveSchedules()`. The timeline renders computed segments (first-match minute coverage), but edits need to modify the underlying `ScheduleEntry[]` array.

The key challenge: the timeline shows *computed coverage* (first-match per minute), but edits must modify *schedule entries* (which have day bitmasks, not per-day data). A visual edit on Monday's timeline needs to find or create the right `ScheduleEntry`.

## Goals / Non-Goals

**Goals:**
- Click on a gap → add a new schedule segment (default: Normal, 2h block, active for that day group)
- Drag segment borders → adjust start/end time (15-min snap)
- Click segment → cycle level (Off→Reduced→Normal→Party→Off)
- All changes reflected immediately in the timeline and saved on "Save All Schedules"
- Works on desktop (mouse); mobile tap support is nice-to-have

**Non-Goals:**
- Drag to move entire segments (only borders)
- Multi-day drag (edit one day at a time)
- Undo/redo
- Real-time save on each drag (batch save with the existing button)

## Decisions

### 1. Edit model: modify entries, re-render timeline

Edits mutate the `entries[]` state in `SchedulesTab` and the timeline re-renders from the updated entries. The timeline component receives `entries` + `onEntriesChange` callback.

### 2. Click on gap → add segment

When clicking a gap area on a specific day:
1. Compute the click position as minutes (0-1440)
2. Snap to 15-min grid
3. Create a new `ScheduleEntry` with a 2-hour block centered on the click position
4. Set `activeDays` to the day group the clicked day belongs to (weekdays if Mon-Fri, weekend if Sat-Sun, or just that day)
5. Set `ventLevel = 2` (Normal) as default
6. Add to `entries[]`, trigger re-render

### 3. Drag border → adjust time

Each segment boundary gets a drag handle (thin invisible div, wide hit area). On `mousedown`:
1. Identify which entry and which edge (start or end) is being dragged
2. On `mousemove`: compute new time from mouse X position, snap to 15-min
3. Update the entry's `startHour/startMinute` or `endHour/endMinute`
4. On `mouseup`: finalize

Challenge: the timeline shows *computed* segments, but we need to map back to the *source entry*. Solution: when rendering, track which source entry each segment came from.

### 4. Click segment → cycle level

Click (not drag) on a segment: find the source entry, cycle `ventLevel` through 0→1→2→3→0.

### 5. Snap to 15-minute grid

All drag positions snap to the nearest 15 minutes. This prevents impractical times like 08:07 and makes the interaction feel clean.

### 6. Source entry tracking

Extend the segment computation to track which `ScheduleEntry` index produced each covered minute. Store this alongside the segment data so click/drag handlers can map visual position → source entry.

### 7. Day group for new segments

When adding a segment by clicking on a day:
- If clicked day is Mon-Fri: set `activeDays = 0x1F` (weekdays)
- If clicked day is Sat-Sun: set `activeDays = 0x60` (weekend)

This matches the common pattern. The user can refine days via the edit dialog.
