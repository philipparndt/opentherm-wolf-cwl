## ADDED Requirements

### Requirement: Home page header reflects active mode
The home screen header SHALL display "Scheduled", "Manual", or "Ventilation" depending on the current control mode.

#### Scenario: Schedule is active
- **WHEN** `schedule_active` is true AND `schedule_override` is false
- **THEN** the header SHALL display "Scheduled" (or "Zeitplan" in German)

#### Scenario: Manual override is active
- **WHEN** `schedule_override` is true
- **THEN** the header SHALL display "Manual" (or "Manuell" in German)

#### Scenario: No schedule and no override
- **WHEN** `schedule_active` is false AND `schedule_override` is false
- **THEN** the header SHALL display "Ventilation" (or "Lüftung" in German)

### Requirement: Mode suffix removed from info line
The info line on the home screen SHALL NOT include the " M" or " S" suffix since the header already communicates the mode.

#### Scenario: Info line content
- **WHEN** the home screen is displayed in normal mode
- **THEN** the info line SHALL show only percentage and bypass mode (e.g. "65%  Winter")
