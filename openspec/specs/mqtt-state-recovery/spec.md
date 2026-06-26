## ADDED Requirements

### Requirement: Enthalpy history is persisted in the snapshot

The retained MQTT history snapshot SHALL include a downsampled, epoch-stamped list of the indoor
and outdoor enthalpy history, emitted alongside the existing temperature and humidity point
lists. Points SHALL be emitted only where at least one enthalpy channel has data, and the
snapshot builder SHALL reserve enough buffer capacity for the added list.

#### Scenario: Snapshot includes enthalpy

- **WHEN** the snapshot is built and enthalpy history exists
- **THEN** the snapshot contains an enthalpy point list with epoch-stamped `[min,max]`/`null`
  pairs for the indoor and outdoor channels

#### Scenario: No enthalpy data

- **WHEN** no enthalpy history exists at snapshot time
- **THEN** the enthalpy list is empty and the rest of the snapshot is unchanged

### Requirement: Enthalpy history is restored from the snapshot

Recovery SHALL re-bin the enthalpy points onto the live 5-minute grid by their epochs (as it
does for humidity), forward-fill gaps into continuous curves, and bound each section's parse so
enthalpy points are not mistaken for humidity, temperature, or bypass entries. A snapshot
produced by firmware that predates enthalpy (no enthalpy list) SHALL restore successfully with
the temperature and humidity history intact and no enthalpy data.

#### Scenario: Enthalpy round-trips through a snapshot

- **WHEN** a snapshot containing enthalpy points is restored against a valid clock
- **THEN** the indoor and outdoor enthalpy history is reconstructed on the 5-minute grid and
  forward-filled into continuous curves

#### Scenario: Backward-compatible restore

- **WHEN** a snapshot without an enthalpy list is restored
- **THEN** temperature and humidity history restore as before and no enthalpy data is added,
  with no parse error
