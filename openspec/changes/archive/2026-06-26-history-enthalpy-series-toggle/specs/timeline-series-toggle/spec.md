## ADDED Requirements

### Requirement: User can toggle timeline series and overlays

The timeline SHALL let the user show or hide each individual data curve (supply temperature,
exhaust temperature, indoor humidity, outdoor humidity, indoor enthalpy, outdoor enthalpy) and
each overlay layer (level-change markers, bypass state strip) independently, using the existing
legend entries as the controls.

#### Scenario: Hide a curve

- **WHEN** the user toggles a visible curve off
- **THEN** that curve and its current-point marker are removed from the plot while the other
  curves remain unchanged

#### Scenario: Show a hidden curve

- **WHEN** the user toggles a hidden curve back on
- **THEN** that curve and its current-point marker are drawn again

#### Scenario: Toggle an overlay layer

- **WHEN** the user toggles the level-change markers or the bypass strip off
- **THEN** that overlay is removed from the plot while the data curves are unaffected

### Requirement: Axes follow visible series

The timeline SHALL show a value axis only while at least one series that uses it is visible, and
SHALL scale each axis to its currently visible series.

#### Scenario: Axis hidden with its series

- **WHEN** all series belonging to an axis are toggled off
- **THEN** that axis and its labels are not drawn

#### Scenario: Axis rescales to visible series

- **WHEN** some but not all series on an axis are hidden
- **THEN** the axis remains and scales to the data of the still-visible series

### Requirement: Toggle selection persists across reloads

The timeline SHALL remember the user's show/hide selection in the browser and restore it on the
next load. The stored selection SHALL be read defensively so that a series not present in the
stored data defaults to visible.

#### Scenario: Selection restored after reload

- **WHEN** the user has hidden some series and reloads the page
- **THEN** the same series are hidden and the rest are shown

#### Scenario: Unknown series defaults to visible

- **WHEN** the stored selection has no entry for a given series
- **THEN** that series defaults to visible
