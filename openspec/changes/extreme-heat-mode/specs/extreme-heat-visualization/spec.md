## ADDED Requirements

### Requirement: Temperature history exposed over HTTP

The system SHALL expose the rolling temperature history (already sampled for the OLED
display) over an HTTP endpoint so the web UI can render it. The response SHALL include the
supply inlet and exhaust inlet series with enough information to plot each over the history
window (per-bucket min/max and the bucket time base).

#### Scenario: Frontend fetches history

- **WHEN** the web UI requests the temperature history endpoint
- **THEN** the system returns the supply and exhaust series as JSON, ordered oldest to
  newest, with the bucket duration and the time of the newest bucket

#### Scenario: Partial history after boot

- **WHEN** fewer than a full window of buckets have been sampled
- **THEN** the response contains only the buckets collected so far, and the frontend can
  still render them without error

### Requirement: Level-change events exposed for markers

The system SHALL record each ventilation level change made by Extreme Heat mode as an event
with a timestamp and the resulting level, and SHALL expose a recent window of these events
over HTTP so they can be drawn as markers on the graph.

#### Scenario: A mode-driven change is recorded

- **WHEN** Extreme Heat mode changes the ventilation level
- **THEN** an event with the change time and the new level is appended to the event log

#### Scenario: Frontend fetches events

- **WHEN** the web UI requests the change-event data
- **THEN** the system returns the recent events covering at least the history window, each
  with a timestamp and the level that was selected

#### Scenario: Bounded storage

- **WHEN** more events occur than the retained capacity
- **THEN** the oldest events are dropped and the newest are kept

### Requirement: Web graph of supply and exhaust temperatures

The web UI SHALL render a graph that plots the supply air and exhaust air temperatures over
the history window, clearly distinguishing the two series and labelling the temperature axis.

#### Scenario: Graph renders both series

- **WHEN** the user views the graph in the web UI
- **THEN** the supply and exhaust temperature lines are both drawn over time and are visually
  distinguishable from each other

#### Scenario: Graph refreshes

- **WHEN** new history data becomes available
- **THEN** the graph updates to include it without a full page reload

### Requirement: Markers when the mode changed the level

The graph SHALL show a marker at each point in time where Extreme Heat mode changed the
ventilation level, and each marker SHALL convey which level was selected.

#### Scenario: Marker at a change point

- **WHEN** Extreme Heat mode changed the level at a given time within the visible window
- **THEN** a marker appears on the graph at that time
- **AND** the marker indicates the level that was selected (e.g. Off / Reduced / Normal / Party)

#### Scenario: No markers when the mode is idle

- **WHEN** Extreme Heat mode made no changes within the visible window
- **THEN** the graph shows the temperature series with no change markers
