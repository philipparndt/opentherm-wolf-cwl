## ADDED Requirements

### Requirement: Publish temperature history as a retained MQTT message

The system SHALL publish the in-RAM temperature history (supply, exhaust, and delta bucket
channels) as a retained MQTT message while MQTT is enabled, and SHALL refresh it whenever a
history bucket rolls over. The payload SHALL contain enough information to reconstruct each
channel's per-bucket min/max in oldest-to-newest order.

#### Scenario: History published on bucket rollover

- **WHEN** MQTT is connected and a temperature-history bucket rolls over
- **THEN** the device publishes the full history snapshot to the retained history topic

#### Scenario: MQTT disabled means no publishing

- **WHEN** MQTT is disabled or not connected
- **THEN** the device does not publish history snapshots and behaviour is otherwise unchanged

### Requirement: Publish extreme-heat mode activation state as a retained MQTT message

The system SHALL publish the extreme-heat mode activation state as a retained MQTT message while
MQTT is enabled, and SHALL refresh it whenever the mode changes the ventilation level or its
enabled state changes. The payload SHALL include the enabled flag, the current decided level, the
last-change timestamp, and the level-change event log used for the graph markers.

#### Scenario: Mode state published on a level change

- **WHEN** MQTT is connected and extreme-heat mode applies a new ventilation level
- **THEN** the device publishes the updated mode-activation snapshot to the retained mode topic
  including the new event

#### Scenario: Mode state published when the mode is toggled

- **WHEN** the extreme-heat mode is enabled or disabled
- **THEN** the device publishes the updated mode-activation snapshot

### Requirement: Recover temperature history from MQTT on boot

While MQTT is enabled, the system SHALL subscribe to the retained history topic on startup and
restore any retained snapshot into the in-RAM history so the graph is populated without waiting
for fresh samples.

#### Scenario: History restored from a retained snapshot

- **WHEN** the device boots with MQTT enabled and a retained history snapshot exists on the broker
- **THEN** the history buckets are restored into RAM in oldest-to-newest order and become visible
  in the web graph

#### Scenario: No snapshot present

- **WHEN** the device boots and no retained history snapshot exists
- **THEN** the history starts empty without error and fills from live samples

### Requirement: Recover extreme-heat mode activation state from MQTT on boot

While MQTT is enabled, the system SHALL subscribe to the retained mode topic on startup and
restore the RAM-only parts of the extreme-heat activation state — the level-change event log,
the current decided level, and the dwell timestamp — from any retained snapshot.

#### Scenario: Markers restored from a retained snapshot

- **WHEN** the device boots with MQTT enabled and a retained mode snapshot exists
- **THEN** the level-change event log is restored so the graph markers reappear

#### Scenario: Enabled flag stays authoritative in config

- **WHEN** a retained mode snapshot is recovered
- **THEN** whether the mode is active is still governed by the persisted configuration, and the
  recovered snapshot only restores the RAM-only decision history and dwell state

### Requirement: Persistence introduces no flash writes

The system SHALL persist the temperature history and mode-activation state only via MQTT and
SHALL NOT write this data to flash or NVS, so that frequent updates do not wear the flash.

#### Scenario: Bucket rollover does not touch flash

- **WHEN** a history bucket rolls over and the snapshot is published
- **THEN** no flash or NVS write occurs as part of persisting the history

### Requirement: Recovery is one-shot and free of feedback loops

The system SHALL apply only the first retained snapshot received per persist topic after boot and
SHALL ignore subsequent messages on those topics, so the device does not re-apply its own
published echoes or overwrite fresher live data with a stale snapshot.

#### Scenario: Later echoes are ignored

- **WHEN** the device has already recovered from a persist topic and then receives a further
  message on that same topic (for example its own subsequent retained publish)
- **THEN** the device ignores it and does not overwrite the current in-RAM data

#### Scenario: Heavy parsing kept off the MQTT callback

- **WHEN** a retained snapshot arrives on the MQTT receive callback
- **THEN** the raw payload is handed to the main loop for parsing and application rather than
  parsed on the callback thread
