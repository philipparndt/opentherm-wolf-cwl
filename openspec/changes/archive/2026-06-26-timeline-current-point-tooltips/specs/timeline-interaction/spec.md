## ADDED Requirements

### Requirement: Current data point is highlighted

The timeline SHALL visually emphasize the most recent populated bucket of each temperature curve
(and of each humidity curve when humidity data is shown) with a distinct current-point marker, so
the "now" end of every series is anchored.

#### Scenario: Latest point marked on temperature curves

- **WHEN** the timeline renders supply and exhaust temperature history
- **THEN** a highlighted current-point marker is drawn at the most recent populated bucket of each
  temperature curve

#### Scenario: Latest point marked on humidity curves

- **WHEN** humidity history is shown on the right-hand axis
- **THEN** a highlighted current-point marker is drawn at the most recent populated bucket of each
  humidity curve

#### Scenario: No data

- **WHEN** there is no temperature history yet
- **THEN** no current-point marker is drawn and the existing "no history" message renders unchanged

### Requirement: Hover tooltip shows the current value

The timeline SHALL display a styled tooltip when the user hovers the current-point marker (or the
plot's right "now" edge), showing the latest supply and exhaust temperature, the latest humidity
values when shown, and the timestamp of that reading.

#### Scenario: Hover the current point

- **WHEN** the user hovers the highlighted current-point marker
- **THEN** a styled tooltip appears showing the current supply and exhaust temperatures (and
  humidity values when shown) together with the time of the reading

#### Scenario: Pointer leaves the current point

- **WHEN** the pointer moves away from the current-point marker
- **THEN** the tooltip is dismissed

### Requirement: Hover tooltip explains level-change markers

The timeline SHALL display a styled tooltip when the user hovers a level-change marker, showing the
ventilation level and the reason the level changed (e.g. temp, cooling, dehumidify, muggy, manual,
schedule, reboot).

#### Scenario: Hover a level-change marker

- **WHEN** the user hovers a level-change marker
- **THEN** a styled tooltip appears naming the level and the reason for the change

#### Scenario: Hover a reboot marker

- **WHEN** the user hovers a marker whose reason is a reboot
- **THEN** the tooltip identifies it as a reboot

### Requirement: Hover tooltip explains bypass spans

The timeline SHALL display a styled tooltip when the user hovers a bypass span in the bypass state
strip, indicating whether the bypass was open (free cooling) or closed (heat recovery) over that
span.

#### Scenario: Hover an open bypass span

- **WHEN** the user hovers a span where the bypass was open
- **THEN** a styled tooltip appears indicating the bypass was open (free cooling)

#### Scenario: Hover a closed bypass span

- **WHEN** the user hovers a span where the bypass was closed
- **THEN** a styled tooltip appears indicating the bypass was closed (heat recovery)

### Requirement: Tooltips are localized

All timeline tooltip text SHALL be available in the application's supported languages (English and
German), reusing the existing level and reason labels.

#### Scenario: Tooltip language follows the UI

- **WHEN** the UI language is set to German
- **THEN** the timeline tooltips render their labels in German
