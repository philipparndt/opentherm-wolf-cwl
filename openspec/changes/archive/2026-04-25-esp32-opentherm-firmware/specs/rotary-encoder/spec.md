## ADDED Requirements

### Requirement: Page navigation via rotation
The firmware SHALL cycle through display pages when the rotary encoder is rotated.

#### Scenario: Rotate clockwise
- **WHEN** the encoder is rotated clockwise
- **THEN** the display advances to the next page (wrapping from last to first)

#### Scenario: Rotate counter-clockwise
- **WHEN** the encoder is rotated counter-clockwise
- **THEN** the display goes to the previous page (wrapping from first to last)

### Requirement: Ventilation level control via encoder
The firmware SHALL allow changing the ventilation level using the encoder button and rotation.

#### Scenario: Enter edit mode
- **WHEN** the encoder button is pressed on the home page
- **THEN** the display enters edit mode with the ventilation level highlighted

#### Scenario: Adjust level in edit mode
- **WHEN** the encoder is rotated while in edit mode
- **THEN** the ventilation level cycles through 0-3

#### Scenario: Confirm level change
- **WHEN** the encoder button is pressed while in edit mode
- **THEN** the new ventilation level is applied and edit mode exits

#### Scenario: Edit mode timeout
- **WHEN** no encoder input is received for 10 seconds while in edit mode
- **THEN** edit mode exits without applying changes

### Requirement: Encoder pin configuration
The firmware SHALL support configurable encoder GPIO pins (CLK, DT, SW) via NVS.

#### Scenario: Custom encoder pins
- **WHEN** the user configures encoder pins via the web UI
- **THEN** the encoder initializes on the configured pins
