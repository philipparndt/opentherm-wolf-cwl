## ADDED Requirements

### Requirement: Home screen header reflects active mode
The home screen SHALL display a distinct header depending on whether the ventilation level is controlled by the schedule, manually overridden, or in default state.

#### Scenario: Schedule is active with no manual override
- **WHEN** `schedule_active` is true AND `schedule_override` is false AND `timed_off_active` is false
- **THEN** the header SHALL display "Schedule" with an inverted style (filled background with text in off-color)

#### Scenario: Manual override is active
- **WHEN** `schedule_override` is true AND `timed_off_active` is false
- **THEN** the header SHALL display "Manual" in normal style (text on transparent background)

#### Scenario: No schedule active and no override
- **WHEN** `schedule_active` is false AND `schedule_override` is false AND `timed_off_active` is false
- **THEN** the header SHALL display "Ventilation" in normal style

### Requirement: Info line omits mode indicator suffix
The small info line on the home screen SHALL NOT include the " M" or " S" mode suffix. The mode is communicated solely through the header.

#### Scenario: Info line content
- **WHEN** the home screen is displayed in any mode (schedule, manual, or default)
- **THEN** the info line SHALL show only the relative ventilation percentage and bypass mode (e.g. "65%  Winter") without a mode letter suffix
