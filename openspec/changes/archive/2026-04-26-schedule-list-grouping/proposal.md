## Why

The schedule list currently shows the raw day bitmask as "Mon, Tue, Wed, Thu, Fri" — verbose and hard to scan. When a schedule applies to all weekdays or all weekend days, it should show a concise group label like "Weekdays" or "Weekend" instead. This makes the list much easier to read at a glance, especially with many schedules.

## What Changes

- Update the schedule list display in the web UI to show grouped day labels:
  - Mon-Fri → "Weekdays"
  - Sat-Sun → "Weekend"
  - Mon-Sun → "Every day"
  - Other combinations → list individual days as before
- Apply the same grouping on the OLED status page schedule description

## Capabilities

### New Capabilities

(none — display-only enhancement)

### Modified Capabilities

(none)

## Impact

- **Modified files**: Web UI `App.tsx` (`daysStr()` function), `scheduler.cpp` (`getActiveScheduleDescription()`)
- No firmware logic changes, no NVS changes
