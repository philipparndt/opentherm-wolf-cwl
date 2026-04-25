## ADDED Requirements

### Requirement: Human-readable value output
The system SHALL display decoded OpenTherm frames in a human-readable format as the default output mode. Each request/response pair SHALL be shown as a single line with the sensor name, interpreted value, and unit.

#### Scenario: Temperature data ID
- **WHEN** a Read-Ack response for "Supply inlet temperature" (ID 80) has value 0x1166
- **THEN** the system SHALL display `Supply inlet temperature:   17.4 °C`

#### Scenario: Relative ventilation percentage
- **WHEN** a Read-Ack response for "Relative ventilation position" (ID 77) has value 0x0033
- **THEN** the system SHALL display `Relative ventilation:       51 %`

#### Scenario: Status flags
- **WHEN** a Read-Ack response for "Status V/H" (ID 70) has value 0x0346
- **THEN** the system SHALL display the individual flag meanings with on/off values, e.g. `Status V/H: fault=off ventilation=on cooling=off ...`

#### Scenario: Fault flags and code
- **WHEN** a Read-Ack response for "Fault flags/code V/H" (ID 72) has value 0x0003
- **THEN** the system SHALL display fault flag bits and the OEM fault code separately

### Requirement: Request/response pairing
The system SHALL pair consecutive request and response packets by matching data IDs. For each pair, the response value SHALL be displayed. Unpaired packets SHALL be shown individually.

#### Scenario: Paired request and response
- **WHEN** a REQ packet for ID 80 is followed by a RSP packet for ID 80
- **THEN** the system SHALL display one line using the response value

#### Scenario: Unpaired request
- **WHEN** a REQ packet has no matching RSP
- **THEN** the system SHALL display the request value with a `[REQ]` label

### Requirement: ESPHome-friendly identifiers
Each displayed value SHALL include a snake_case identifier suitable for ESPHome entity IDs (e.g., `supply_inlet_temperature`, `relative_ventilation`, `exhaust_inlet_temperature`).

#### Scenario: Identifier format
- **WHEN** displaying "Supply inlet temperature"
- **THEN** the identifier `supply_inlet_temperature` SHALL be shown or derivable from the output

### Requirement: Summary header
The readable output SHALL start with a brief summary line showing the number of decoded exchanges and capture duration, without the detailed bit-timing analysis.

#### Scenario: Summary line
- **WHEN** decoding completes with 13 exchanges over 12.76s
- **THEN** the system SHALL display a summary like `Decoded 13 exchanges (12.8s capture)`
