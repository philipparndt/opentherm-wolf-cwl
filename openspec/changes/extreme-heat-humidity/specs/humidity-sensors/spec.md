## ADDED Requirements

### Requirement: Configure indoor and outdoor humidity sensor topics

The system SHALL let the user configure a list of indoor humidity sensor MQTT topics (one or
more) and a single outdoor humidity sensor MQTT topic, and SHALL persist this configuration so
it survives reboots.

#### Scenario: User adds sensor topics

- **WHEN** the user sets one or more indoor topics and an outdoor topic and saves
- **THEN** the configuration is stored persistently and used to subscribe after the next connect

#### Scenario: Configuration persists across reboot

- **WHEN** the device reboots
- **THEN** the previously configured sensor topics are still in effect

### Requirement: Ingest humidity, temperature and pressure from MQTT sensors

The system SHALL subscribe to every configured sensor topic while MQTT is enabled and SHALL parse
the `humidity`, `temperature` and `pressure` fields from each sensor's JSON payload, storing the
latest values per sensor together with the time they were received. The `temperature` and
`pressure` fields MAY be absent; `humidity` is required for a reading to be usable.

#### Scenario: Sensor publishes a reading

- **WHEN** a configured sensor topic receives a JSON payload containing a `humidity` value
- **THEN** the system stores that humidity (and temperature and pressure, if present) as the latest
  reading for that sensor with the current timestamp

#### Scenario: Payload with extra fields

- **WHEN** a payload contains additional fields (battery, linkquality, voltage, …)
- **THEN** the system extracts only humidity, temperature and pressure and ignores the rest without error

#### Scenario: Payload without pressure

- **WHEN** a sensor payload has humidity and temperature but no `pressure`
- **THEN** the reading is still usable and the system relies on the last known ambient pressure (or a
  standard default) for calculations that need it

#### Scenario: Malformed payload

- **WHEN** a payload is not valid JSON or has no `humidity` field
- **THEN** the system ignores it and keeps the previous reading

### Requirement: Track ambient pressure

The system SHALL maintain a current ambient air pressure taken from whichever sensor reports a
`pressure` field, and SHALL fall back to a standard sea-level-equivalent default when no sensor has
ever reported pressure, so enthalpy can be computed correctly at the local altitude.

#### Scenario: A sensor reports pressure

- **WHEN** any configured sensor publishes a `pressure` value
- **THEN** the system updates the current ambient pressure from it

#### Scenario: No sensor reports pressure

- **WHEN** no configured sensor has ever reported pressure
- **THEN** the system uses a standard default pressure for calculations

### Requirement: Track sensor freshness

The system SHALL treat a sensor reading as stale once it is older than a defined freshness window,
so that decisions and displays can distinguish live sensors from ones that have gone offline
(e.g. a dead battery).

#### Scenario: Reading goes stale

- **WHEN** no new payload has arrived from a sensor within the freshness window
- **THEN** that sensor's reading is marked stale and is not used as live data

#### Scenario: Fresh reading clears stale state

- **WHEN** a stale sensor publishes a new reading
- **THEN** it is considered fresh again

### Requirement: Expose current sensor readings

The system SHALL expose the current per-sensor humidity/temperature readings and their fresh/stale
status so the web UI can display them.

#### Scenario: Status includes sensor readings

- **WHEN** the web UI requests status
- **THEN** the response includes each configured sensor's latest humidity, temperature (if any),
  pressure (if any), and whether it is fresh or stale, plus the current ambient pressure in use
