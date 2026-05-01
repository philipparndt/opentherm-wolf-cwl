## ADDED Requirements

### Requirement: Schedule option in level selection cycle
The rotary encoder level selection on the home page SHALL include a "Schedule" option after "Party", cycling as: Off → Reduced → Normal → Party → Schedule → Off (wrapping in both directions).

#### Scenario: Rotating forward past Party
- **WHEN** the edit level is at Party (3) and the user rotates forward
- **THEN** the edit level SHALL advance to Schedule (displayed as "Schedule")

#### Scenario: Rotating backward past Off
- **WHEN** the edit level is at Off (0) and the user rotates backward
- **THEN** the edit level SHALL wrap to Schedule

#### Scenario: Rotating forward past Schedule
- **WHEN** the edit level is at Schedule and the user rotates forward
- **THEN** the edit level SHALL wrap to Off (0)

### Requirement: Selecting Schedule clears override
When the user confirms the "Schedule" option, the system SHALL clear the manual override and return to schedule-controlled ventilation.

#### Scenario: Confirming Schedule while schedule is active
- **WHEN** the user selects "Schedule" and a schedule program is currently active
- **THEN** `schedule_override` SHALL be set to false AND the scheduler SHALL apply the current schedule level on its next evaluation

#### Scenario: Confirming Schedule while no schedule is active
- **WHEN** the user selects "Schedule" and no schedule program is currently active
- **THEN** `schedule_override` SHALL be set to false AND the system SHALL use the configured default ventilation level

#### Scenario: Confirming Schedule while timed-off is active
- **WHEN** the user selects "Schedule" and a timed-off countdown is active
- **THEN** the timed-off SHALL be cancelled AND `schedule_override` SHALL be set to false

### Requirement: Manual override persists across schedule transitions
When the user explicitly selects a ventilation level (Off/Reduced/Normal/Party), the override SHALL persist until the user explicitly selects "Schedule" or clears override via the web UI.

#### Scenario: Schedule time slot changes while override is active
- **WHEN** `schedule_override` is true AND the active schedule transitions to a new time slot
- **THEN** `schedule_override` SHALL remain true AND the manually selected level SHALL remain active

#### Scenario: User selects Reduced while Normal was scheduled
- **WHEN** the schedule is running Normal and the user selects Reduced
- **THEN** Reduced SHALL remain active even after the schedule transitions to the next time slot
