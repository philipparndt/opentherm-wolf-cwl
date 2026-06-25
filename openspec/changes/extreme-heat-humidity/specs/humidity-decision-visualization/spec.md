## ADDED Requirements

### Requirement: Combined temperature and humidity history chart

The web UI SHALL show temperature and humidity together over the history window so the inputs to
the decision can be read at a glance, with temperature and humidity on clearly distinguished
scales and each series labelled.

#### Scenario: Chart shows both temperature and humidity

- **WHEN** the user views the chart
- **THEN** indoor and outdoor temperature and humidity are plotted over time, visually distinct and
  with a legend identifying each series and its scale

#### Scenario: Humidity series absent

- **WHEN** no humidity data is available
- **THEN** the chart still shows the temperature series without error

### Requirement: Change markers annotated with the decision reason

The graph's level-change markers SHALL convey not only the level chosen but the reason it changed
(temperature delta, cooling assist, moisture protection, or muggy suppression), so a change can be
understood without guessing.

#### Scenario: Marker shows reason

- **WHEN** a level change occurred because of moisture protection
- **THEN** its marker indicates that reason as well as the new level

### Requirement: Live decision-explainer panel

The web UI SHALL show a live panel that explains the current decision: the current inputs
(indoor/outdoor temperature and humidity, derived absolute humidity and enthalpy), which rule is
currently driving the level, and the resulting level, in plain language.

#### Scenario: Explainer reflects the active rule

- **WHEN** the mode is currently ventilating for moisture protection
- **THEN** the panel shows the moisture-protection rule as active, the relevant readings, and the
  resulting level

#### Scenario: Explainer in temperature-only fallback

- **WHEN** humidity data is stale or missing
- **THEN** the panel indicates the decision is using the temperature-only fallback

### Requirement: Per-sensor status display

The web UI SHALL list each configured humidity sensor with its latest reading and whether it is
fresh or stale, so a dead or misconfigured sensor is obvious.

#### Scenario: A sensor is stale

- **WHEN** a configured sensor has not reported within the freshness window
- **THEN** the UI shows that sensor as stale
