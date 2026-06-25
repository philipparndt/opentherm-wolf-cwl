## ADDED Requirements

### Requirement: Enable and persist Extreme Heat mode

The system SHALL provide an Extreme Heat mode that can be turned on and off by the user,
and SHALL persist the enabled/disabled state across reboots. While the mode is disabled,
the system SHALL NOT alter ventilation behaviour in any way.

#### Scenario: User enables the mode

- **WHEN** the user enables Extreme Heat mode
- **THEN** the enabled state is stored in persistent config
- **AND** the automatic control loop begins driving the ventilation level

#### Scenario: State survives reboot

- **WHEN** Extreme Heat mode is enabled and the device reboots
- **THEN** the mode is still enabled after boot without user interaction

#### Scenario: Disabled mode is inert

- **WHEN** Extreme Heat mode is disabled
- **THEN** the system never changes `requested_vent_level` on account of this feature
- **AND** schedules, manual control, and timed-off behave exactly as before

### Requirement: Automatic level selection from supply-vs-exhaust temperature

The system SHALL select the ventilation level from the difference delta (supply inlet minus
exhaust inlet) while Extreme Heat mode is enabled and both temperatures are valid. The supply
inlet temperature is OpenTherm ID 80 and the exhaust inlet temperature is OpenTherm ID 82.
The system SHALL map delta to levels as follows: delta above +0.5 C selects Off; delta above
0 C up to and including +0.5 C selects Reduced (low); delta from -1.0 C up to but not including
0 C selects Normal; and delta below -1.0 C selects Party.

#### Scenario: Incoming air much hotter than indoor

- **WHEN** supply is more than 0.5 °C above exhaust
- **THEN** the selected level is Off

#### Scenario: Incoming air slightly hotter than indoor

- **WHEN** supply is above exhaust but by 0.5 °C or less
- **THEN** the selected level is Reduced

#### Scenario: Incoming air cooler than indoor

- **WHEN** supply is below exhaust by less than 1.0 °C
- **THEN** the selected level is Normal

#### Scenario: Incoming air much cooler than indoor

- **WHEN** supply is more than 1.0 °C below exhaust
- **THEN** the selected level is Party

#### Scenario: Temperatures not yet available

- **WHEN** either the supply or exhaust temperature is missing or invalid
- **THEN** the system makes no automatic level change and waits for valid readings

### Requirement: Minimum 15-minute decision dwell

A level chosen by Extreme Heat mode SHALL be held for at least 15 minutes before the mode
may change the level again, so that readings hovering near a band boundary do not cause
rapid toggling.

#### Scenario: Change blocked within dwell window

- **WHEN** the mode set a level less than 15 minutes ago
- **AND** the current temperatures would select a different level
- **THEN** the mode keeps the current level and does not change it yet

#### Scenario: Change allowed after dwell window

- **WHEN** at least 15 minutes have passed since the last mode-driven change
- **AND** the current temperatures select a different level
- **THEN** the mode applies the new level and restarts the 15-minute timer

#### Scenario: Same level needs no change

- **WHEN** the dwell window has elapsed
- **AND** the selected level equals the current level
- **THEN** no change is applied and the dwell timer is not reset

### Requirement: Interaction with manual override and timed-off

Extreme Heat mode SHALL NOT fight an explicit timed-off command, and the precedence
between the mode and a manual level change SHALL be well defined and predictable.

#### Scenario: Timed-off takes precedence

- **WHEN** a timed-off period is active
- **THEN** Extreme Heat mode does not raise the level until the timed-off period ends

#### Scenario: Manual change is honoured within the dwell window

- **WHEN** the user manually sets a ventilation level while Extreme Heat mode is enabled
- **THEN** the manual level is applied immediately
- **AND** the mode treats this as the current decision and waits the full dwell window
  before overriding it
