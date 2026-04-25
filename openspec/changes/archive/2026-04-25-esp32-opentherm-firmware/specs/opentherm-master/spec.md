## ADDED Requirements

### Requirement: OpenTherm master initialization
The firmware SHALL initialize the OpenTherm library as master using configurable input and output GPIO pins (default: input=21, output=22 for DIYLess shield).

#### Scenario: Successful initialization
- **WHEN** the ESP32 boots with valid OpenTherm pin configuration
- **THEN** the OpenTherm library is initialized in master mode and begins polling

#### Scenario: Pin configuration from NVS
- **WHEN** the user has configured custom OT pins via the web UI
- **THEN** the firmware uses the NVS-stored pin numbers instead of defaults

### Requirement: Polling cycle matches BM remote
The firmware SHALL poll the Wolf CWL in a repeating cycle with ~1 second between each request, sending all required data IDs per cycle.

#### Scenario: Standard polling cycle
- **WHEN** a polling cycle starts
- **THEN** the firmware sends requests for IDs 2, 70, 71, 72, 74, 77, 80, 82, 89, 126, 127 in sequence with ~1 second spacing

#### Scenario: TSP index rotation
- **WHEN** Data ID 89 is polled
- **THEN** the TSP index increments each cycle, cycling through indices 0-68

### Requirement: Ventilation level control
The firmware SHALL write the current ventilation level (0-3) to Data ID 71 each polling cycle.

#### Scenario: Set ventilation level
- **WHEN** a ventilation level change is requested (via MQTT or encoder)
- **THEN** the next Write-Data to ID 71 uses the new level value (0=Off, 1=Reduced, 2=Normal, 3=Party)

#### Scenario: Invalid level rejected
- **WHEN** a ventilation level outside 0-3 is requested
- **THEN** the request is ignored and the current level is maintained

### Requirement: Bypass damper control
The firmware SHALL control the bypass damper by setting/clearing bit 1 in the HI byte of the Status V/H (ID 70) Read-Data request.

#### Scenario: Open bypass
- **WHEN** bypass open is requested
- **THEN** bit 1 of the HI byte in the ID 70 request is set to 1

#### Scenario: Close bypass
- **WHEN** bypass close is requested
- **THEN** bit 1 of the HI byte in the ID 70 request is cleared to 0

### Requirement: Filter reset
The firmware SHALL support triggering a filter timer reset by setting bit 4 in the HI byte of the ID 70 request.

#### Scenario: Filter reset triggered
- **WHEN** a filter reset command is received
- **THEN** bit 4 is set in the ID 70 HI byte for one polling cycle, then automatically cleared

### Requirement: Status and sensor data parsing
The firmware SHALL parse all response frames and store the current values in a data model.

#### Scenario: Temperature parsing
- **WHEN** a Read-Ack response is received for ID 80 or 82
- **THEN** the f8.8 value is decoded to a float temperature in °C

#### Scenario: Status flags parsing
- **WHEN** a Read-Ack response is received for ID 70
- **THEN** the HI byte is decoded into fault, ventilation/bypass, cooling, dhw, filter, and diag flags

#### Scenario: Relative ventilation parsing
- **WHEN** a Read-Ack response is received for ID 77
- **THEN** the HI byte is stored as relative ventilation percentage (0-100)

#### Scenario: TSP value parsing
- **WHEN** a Read-Ack response is received for ID 89
- **THEN** the index and value are stored in the TSP register cache

### Requirement: Probe additional data IDs
The firmware SHALL probe data IDs 78, 79, 81, 83, 84, 85, 87, 88 at startup to discover additional sensors.

#### Scenario: Supported ID discovered
- **WHEN** a probe request receives a Read-Ack response
- **THEN** the data ID is added to the regular polling cycle

#### Scenario: Unsupported ID skipped
- **WHEN** a probe request receives Unknown-DataId or no response
- **THEN** the data ID is excluded from the polling cycle
