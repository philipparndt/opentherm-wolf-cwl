## ADDED Requirements

### Requirement: Timeline is independent of Extreme Heat mode

The temperature timeline SHALL be a standalone feature that displays the supply and exhaust
temperature history (and humidity history when available) regardless of whether Extreme Heat
mode is enabled. The timeline SHALL NOT require Extreme Heat mode to be on in order to render
its temperature data.

#### Scenario: Timeline renders with Extreme Heat disabled

- **WHEN** Extreme Heat mode is disabled and temperature history exists
- **THEN** the timeline still renders the supply and exhaust temperature curves

#### Scenario: Timeline renders with Extreme Heat enabled

- **WHEN** Extreme Heat mode is enabled
- **THEN** the timeline renders the same temperature history, unchanged by the mode being on

### Requirement: Extreme-heat level changes are an optional overlay

The system SHALL present extreme-heat level-change events as an optional overlay on the
timeline, not as a precondition for the timeline. When there are no such events, the timeline
SHALL still render its temperature and humidity history.

#### Scenario: No level-change events

- **WHEN** the timeline has temperature history but no extreme-heat level-change events
- **THEN** the temperature curves render and no level-change markers or marker legend appear

#### Scenario: Level-change events present

- **WHEN** extreme-heat level-change events exist within the history window
- **THEN** they are drawn as markers overlaid on the timeline with their existing legend
