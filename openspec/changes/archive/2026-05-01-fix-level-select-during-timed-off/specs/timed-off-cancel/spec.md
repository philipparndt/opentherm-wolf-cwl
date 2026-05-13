## ADDED Requirements

### Requirement: Selecting a level during timed-off applies that level
When the user selects a ventilation level while timed-off is active, the selected level SHALL be applied and remain active after the timed-off is cancelled.

#### Scenario: Select Reduced while Off timer is running
- **WHEN** timed-off is active and the user selects "Reduced" via the encoder
- **THEN** the ventilation SHALL switch to Reduced with manual override active

#### Scenario: Select Party while Off timer is running
- **WHEN** timed-off is active and the user selects "Party" via the encoder
- **THEN** the ventilation SHALL switch to Party with manual override active

### Requirement: cancel_timed_off only stops the timer
The `cancel_timed_off()` function SHALL only deactivate the timed-off countdown. It SHALL NOT modify `requested_vent_level` or `schedule_override`.

#### Scenario: Timer cancelled preserves current state
- **WHEN** `cancel_timed_off()` is called
- **THEN** `timed_off_active` SHALL become false AND `requested_vent_level` and `schedule_override` SHALL remain unchanged
