## ADDED Requirements

### Requirement: MQTT sensor publishing
The firmware SHALL publish all sensor values to MQTT topics after each polling cycle completes.

#### Scenario: Temperature published
- **WHEN** a polling cycle completes with valid temperature readings
- **THEN** supply inlet and exhaust inlet temperatures are published as float strings to `{topic}/temperature/supply_inlet` and `{topic}/temperature/exhaust_inlet`

#### Scenario: Ventilation state published
- **WHEN** a polling cycle completes
- **THEN** the current ventilation level is published to `{topic}/ventilation/level` and relative ventilation % to `{topic}/ventilation/relative`

#### Scenario: Status flags published
- **WHEN** a polling cycle completes
- **THEN** fault, filter, and bypass states are published as "0" or "1" to `{topic}/status/fault`, `{topic}/status/filter`, `{topic}/status/bypass`

### Requirement: MQTT command subscription
The firmware SHALL subscribe to command topics and apply changes to the OpenTherm control state.

#### Scenario: Set ventilation level via MQTT
- **WHEN** a message "2" is received on `{topic}/set/level`
- **THEN** the ventilation level is set to Normal (2)

#### Scenario: Toggle bypass via MQTT
- **WHEN** a message "1" is received on `{topic}/set/bypass`
- **THEN** the bypass damper is commanded open

#### Scenario: Filter reset via MQTT
- **WHEN** a message "1" is received on `{topic}/set/filter_reset`
- **THEN** the filter timer reset is triggered for one polling cycle

### Requirement: MQTT bridge state
The firmware SHALL publish bridge availability using Last Will and Testament.

#### Scenario: Online announcement
- **WHEN** the MQTT client connects
- **THEN** "online" is published (retained) to `{topic}/bridge/state` and firmware version to `{topic}/bridge/version`

#### Scenario: Offline detection
- **WHEN** the MQTT client disconnects unexpectedly
- **THEN** the broker publishes the LWT message "offline" to `{topic}/bridge/state`

### Requirement: MQTT configuration
The firmware SHALL support configurable MQTT broker, port, username, password, and base topic via NVS.

#### Scenario: MQTT settings persisted
- **WHEN** MQTT settings are saved via the web UI
- **THEN** the settings are stored in NVS and used on next connection attempt
