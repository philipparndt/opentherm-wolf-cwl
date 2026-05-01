## ADDED Requirements

### Requirement: WeekTimeline always visible
The WeekTimeline component SHALL be rendered regardless of whether schedule entries exist. It SHALL NOT be conditionally hidden when the entries array is empty.

#### Scenario: No schedule entries configured
- **WHEN** the schedule entries array is empty
- **THEN** the WeekTimeline SHALL still be displayed with the time axis, day labels, current-time indicator, and full-width gap segments

#### Scenario: Current time visible on empty timeline
- **WHEN** no entries exist and today's day row is visible
- **THEN** the current-time vertical line SHALL still be displayed at the correct position

### Requirement: Empty timeline supports adding schedules
The empty timeline with gap segments SHALL support the existing click-to-add interaction.

#### Scenario: Clicking gap on empty timeline
- **WHEN** the user clicks on a gap segment in the empty timeline
- **THEN** a new schedule entry SHALL be created for that time range and day (existing gap-click behavior)
