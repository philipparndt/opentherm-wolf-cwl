## ADDED Requirements

### Requirement: Timed off activation via encoder
The firmware SHALL implement a two-stage encoder flow for Off mode: selecting Off enters a duration substage where rotating the encoder sets the off duration in hours (1-99), and pressing confirms.

#### Scenario: Enter timed off
- **WHEN** the user enters edit mode and rotates to Off, then rotates to set 3 hours, then presses to confirm
- **THEN** ventilation is set to Off for 3 hours

#### Scenario: Cancel during selection
- **WHEN** the user is in the Off duration substage and rotates past 1h back
- **THEN** the edit returns to level 3 (Party) with no timer

#### Scenario: Select non-off level
- **WHEN** the user selects Reduced, Normal, or Party and presses to confirm
- **THEN** the level is set immediately with no timer (existing behavior)

### Requirement: Schedule suspension during timed off
The firmware SHALL suspend all ventilation schedule evaluation while timed off is active.

#### Scenario: Schedule suspended
- **WHEN** timed off is active
- **THEN** the scheduler does not change the ventilation level

#### Scenario: Schedule resumes after expiry
- **WHEN** the timed off timer expires
- **THEN** ventilation returns to Normal (level 2) and schedules resume evaluation

### Requirement: Timed off expiry
The firmware SHALL automatically return to Normal (level 2) when the timed off duration expires.

#### Scenario: Timer expires
- **WHEN** the timed off timer reaches zero
- **THEN** ventilation level is set to Normal and `timedOffActive` is cleared

### Requirement: Timed off cancellation
The firmware SHALL cancel timed off when a non-zero ventilation level is set via any input (MQTT, encoder, web UI).

#### Scenario: Cancel via MQTT
- **WHEN** timed off is active and `set/level` receives value "2"
- **THEN** timed off is cancelled and level is set to Normal

#### Scenario: Cancel via encoder
- **WHEN** timed off is active and the user sets a new level via the encoder
- **THEN** timed off is cancelled

### Requirement: OLED countdown display
The OLED home page SHALL show the remaining time during timed off.

#### Scenario: Countdown displayed
- **WHEN** timed off is active with 2h 45m remaining
- **THEN** the home page shows "Off" with "Resumes in 2h 45m" below

### Requirement: MQTT timed off command and status
The firmware SHALL accept a timed off command via MQTT and publish the timer state.

#### Scenario: Start via MQTT
- **WHEN** a message "3" is received on `{topic}/set/off_timer`
- **THEN** timed off is activated for 3 hours

#### Scenario: Status published
- **WHEN** timed off is active
- **THEN** `{topic}/off_timer/active` publishes "1" and `{topic}/off_timer/remaining` publishes the remaining minutes

### Requirement: Web UI timed off status
The web UI Status tab SHALL show the timed off state with a cancel button.

#### Scenario: Cancel from web UI
- **WHEN** timed off is active and the user clicks Cancel
- **THEN** timed off is cancelled and ventilation returns to Normal
