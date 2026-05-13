## ADDED Requirements

### Requirement: Pending level shown with prefix
When the requested ventilation level differs from the CWL-confirmed level, the home screen SHALL display the requested level with a ">" prefix.

#### Scenario: Level switch in progress
- **WHEN** `requested_vent_level` is 1 (Reduced) AND `cwl_data.ventilation_level` is 2 (Normal)
- **THEN** the display SHALL show "> Reduced" in the large centered text area

#### Scenario: Level confirmed
- **WHEN** `requested_vent_level` equals `cwl_data.ventilation_level`
- **THEN** the display SHALL show the level name without any prefix

#### Scenario: Timed-off excluded
- **WHEN** `timed_off_active` is true
- **THEN** the pending indicator logic SHALL NOT apply (timed-off screen shows "Off" with countdown regardless)
