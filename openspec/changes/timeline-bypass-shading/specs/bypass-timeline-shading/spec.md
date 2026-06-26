## ADDED Requirements

### Requirement: Record bypass state across the history window

The system SHALL maintain a bounded log of bypass transitions, each recording the wall-clock
epoch and whether the bypass became open or closed. A transition SHALL be recorded only when
the requested bypass state actually changes, and SHALL be recorded regardless of which control
path changed it (Extreme Heat mode, the calendar schedule, or a manual web/MQTT/encoder
change). The log SHALL be bounded, dropping the oldest transitions when full.

#### Scenario: Bypass opens

- **WHEN** the requested bypass state changes from closed to open
- **THEN** a transition `{epoch, open: true}` is appended to the log

#### Scenario: Bypass closes

- **WHEN** the requested bypass state changes from open to closed
- **THEN** a transition `{epoch, open: false}` is appended to the log

#### Scenario: No change, no transition

- **WHEN** the requested bypass state is re-set to the value it already holds
- **THEN** no new transition is appended

### Requirement: Expose bypass transitions to the timeline

The system SHALL expose the bypass transition log to the web UI through the existing
`/api/history` response, as a list of `{epoch, open}` entries on the same time basis as the
temperature data, without adding a new endpoint.

#### Scenario: History payload includes bypass transitions

- **WHEN** the web UI requests `/api/history`
- **THEN** the response includes a `bypass` array of `{epoch, open}` transitions

#### Scenario: Older firmware snapshot without bypass data

- **WHEN** the history payload contains no bypass transition data
- **THEN** the timeline renders the curves with no background shading and does not error

### Requirement: Persist bypass history across reboots

The system SHALL include the bypass transition log in the retained temperature-history
snapshot so the shading is restored after a reboot, alongside the temperature curves it
annotates.

#### Scenario: Shading survives a reboot

- **WHEN** the device reboots and restores its retained temperature-history snapshot
- **THEN** the bypass transitions within the window are restored and the shading reappears

### Requirement: Two-tone bypass background shading aligned to the time axis

The timeline SHALL draw its chart background as spans aligned to the time axis, tinted by the
bypass state during each span: a base gray tone where the bypass was closed and a slightly
brighter gray tone where the bypass was open. The shading SHALL be drawn behind the gridlines,
series, and markers, and SHALL be accompanied by a legend entry explaining the two tones.

#### Scenario: Open span is brighter

- **WHEN** the bypass was open during a span of the window
- **THEN** that span's background is filled with the brighter gray tone

#### Scenario: Closed span is the base tone

- **WHEN** the bypass was closed during a span of the window
- **THEN** that span's background is filled with the base gray tone

#### Scenario: Span before the earliest known transition

- **WHEN** the window extends earlier than the oldest recorded transition
- **THEN** that leading span is tinted as the inverse of the oldest transition's new state

#### Scenario: Shading does not obscure the data

- **WHEN** background shading is drawn
- **THEN** it is rendered behind the temperature curves, markers, and gridlines, which remain
  fully legible
