## ADDED Requirements

### Requirement: Hold-down/cooling decision is stable around equal energy

The system SHALL apply hysteresis to the energy-based decision so that the choice between
**reducing ventilation** (outdoor air not worth importing) and **ventilating to cool** (outdoor air
lower-energy) does not change while the enthalpy delta `Δh = h_out − h_in` merely wanders across
zero within sensor noise. The system SHALL switch into cooling only when outdoor air is at least a
configured margin lower-energy than indoor air, and SHALL release back to reducing only when outdoor
air is no longer lower-energy; between those two thresholds it SHALL hold its current choice. The
Off (outdoor much higher-energy) and Party (outdoor much lower-energy) bands are unaffected.

#### Scenario: Energy delta wandering across zero does not flip the level

- **WHEN** the mode is reducing ventilation and the enthalpy delta moves from slightly positive to
  slightly negative but stays within the neutral band around zero
- **THEN** the mode keeps reducing ventilation and does not switch to cooling

#### Scenario: Switch to cooling only once outdoor air is clearly lower-energy

- **WHEN** the mode is reducing ventilation and the outdoor enthalpy falls below the indoor enthalpy
  by more than the enter-cooling margin
- **THEN** the mode switches to ventilating for cooling

#### Scenario: Release from cooling only once outdoor air is no longer lower-energy

- **WHEN** the mode is ventilating for cooling and the outdoor enthalpy rises back to or above the
  indoor enthalpy
- **THEN** the mode stops cooling and returns to reducing ventilation

#### Scenario: Much-lower-energy outdoor air still goes to the strongest cooling band

- **WHEN** the outdoor enthalpy is far below the indoor enthalpy (beyond the Party margin)
- **THEN** the mode selects the strongest cooling level, as before, regardless of the hysteresis band

### Requirement: Cooling-assist reason reflects actual cooling

The system SHALL report the cooling-assist reason only when it has entered the cooling state past
the enter-cooling margin, so a marginal crossing of equal energy is not labelled as cooling. While
the decision is held in the neutral band having previously been reducing ventilation, the reason
SHALL remain the hold-down (muggy-suppression) reason rather than cooling-assist.

#### Scenario: Marginal crossing is not reported as cooling

- **WHEN** the enthalpy delta is only marginally negative and inside the neutral band, and the mode
  was reducing ventilation
- **THEN** the displayed reason stays muggy-suppression and is not reported as cooling-assist
