## ADDED Requirements

### Requirement: Enthalpy history is recorded

The system SHALL record indoor and outdoor specific enthalpy (kJ/kg) into the history at
5-minute resolution, reusing the already-computed enthalpy value from the humidity decision
inputs. Enthalpy SHALL be folded only when a fresh value is available for that side, and SHALL
share the humidity bucket cadence so its columns align with the humidity history.

#### Scenario: Enthalpy folded when fresh inputs exist

- **WHEN** at least one fresh sensor exists on a side so an enthalpy value is computed
- **THEN** that side's enthalpy is folded into the current 5-minute history bucket

#### Scenario: No fresh inputs on a side

- **WHEN** a side has no fresh sensor and no enthalpy value can be computed
- **THEN** nothing is folded for that side and its enthalpy history simply has no data for that
  bucket

#### Scenario: Buckets advance on the humidity cadence

- **WHEN** a 5-minute humidity bucket boundary is crossed
- **THEN** the indoor and outdoor enthalpy buckets advance together with the humidity buckets

### Requirement: Enthalpy is exposed to the web timeline

The `/api/history` payload SHALL include the indoor and outdoor enthalpy series alongside the
existing temperature and humidity series, using the same compact `[min,max]` / `null` bucket
encoding and the humidity slot count and bucket interval.

#### Scenario: Enthalpy present in history payload

- **WHEN** the web client fetches the history endpoint and enthalpy data exists
- **THEN** the payload contains the indoor and outdoor enthalpy arrays in `[min,max]`/`null` form

#### Scenario: Enthalpy absent

- **WHEN** no enthalpy data has been recorded yet
- **THEN** the enthalpy arrays are present but contain only `null` buckets, and the rest of the
  payload is unchanged

### Requirement: Timeline renders enthalpy curves

The timeline SHALL render the indoor and outdoor enthalpy history as curves on their own
auto-scaled value axis (kJ/kg), visually distinct from the temperature and humidity curves, with
a current-point marker at the most recent populated bucket of each enthalpy curve as with the
other series.

#### Scenario: Enthalpy curves drawn when data exists

- **WHEN** the timeline has indoor or outdoor enthalpy history
- **THEN** the enthalpy curves are drawn on their own kJ/kg axis, distinct from temperature and
  humidity, each with a current-point marker at its latest bucket

#### Scenario: No enthalpy data

- **WHEN** there is no enthalpy history
- **THEN** no enthalpy curve, axis, or marker is drawn and the rest of the timeline renders
  unchanged
