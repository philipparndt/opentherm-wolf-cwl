## ADDED Requirements

### Requirement: Decide ventilation by air energy when humidity is available

The system SHALL base the cooling decision on the **enthalpy** (total energy) of the indoor and
outdoor air, rather than temperature alone, whenever extreme-heat mode is enabled and fresh indoor
and outdoor humidity readings are available — because humid air carries more energy than dry air at
the same temperature. Lower-energy outdoor air SHALL favour ventilating; higher-energy (e.g. warm
and humid) outdoor air SHALL favour reducing or stopping ventilation, even if it is not hotter.

#### Scenario: Cooler-feeling but muggy outdoor air does not trigger ventilation

- **WHEN** outdoor air is slightly cooler than indoor but much more humid, so its total energy is
  higher than indoor air
- **THEN** the mode does not raise ventilation to bring that air in

#### Scenario: Warmer but much drier outdoor air assists cooling

- **WHEN** outdoor air is marginally warmer than indoor but far drier, so its total energy is lower
- **THEN** the mode raises ventilation to bring that lower-energy air in, regardless of the bare
  temperature difference

#### Scenario: Enthalpy uses ambient pressure

- **WHEN** the energy (enthalpy) of the air is computed
- **THEN** the current ambient pressure (from a sensor's `pressure` field, or the default) is used,
  so the calculation is correct at the local altitude

### Requirement: Moisture protection is independently enableable

The system SHALL provide moisture protection as its own feature that can be enabled or disabled
independently of extreme-heat mode and SHALL persist that setting. Moisture protection SHALL operate
year-round whenever enabled, regardless of season or whether extreme-heat mode is on.

#### Scenario: Protection on, extreme-heat off

- **WHEN** moisture protection is enabled and extreme-heat mode is disabled
- **THEN** moisture protection still evaluates and can act on indoor humidity

#### Scenario: Setting persists

- **WHEN** the device reboots
- **THEN** the moisture-protection enabled/disabled setting is unchanged

### Requirement: Moisture-protection ventilation

When moisture protection is enabled and indoor air is too humid, the system SHALL turn ventilation on
— regardless of the temperature difference or season — provided the outdoor air is drier in absolute
terms so that ventilating actually lowers indoor humidity. If ventilating would not reduce indoor
humidity (outdoor air is not drier), the system SHALL NOT ventilate for this reason.

#### Scenario: Too humid indoors with drier outside air

- **WHEN** indoor relative humidity is above the protection threshold and outdoor absolute humidity
  is lower than indoor
- **THEN** the system turns ventilation on to expel indoor moisture, regardless of the temperature delta

#### Scenario: Too humid indoors but outside is just as humid

- **WHEN** indoor humidity is high but outdoor absolute humidity is not lower than indoor
- **THEN** the system does not ventilate for moisture protection (it would not help)

### Requirement: Standalone protection yields control when not needed

When moisture protection is acting on its own (extreme-heat mode not enabled), it SHALL only raise
ventilation and SHALL NOT lower it, and once indoor humidity is back within range it SHALL release
control so the normal schedule or manual level resumes. A short hysteresis / the decision dwell SHALL
be applied so it does not toggle rapidly around the threshold.

#### Scenario: Protection releases after drying out

- **WHEN** moisture protection raised the level and indoor humidity later falls back within range
- **THEN** the system stops overriding and the scheduled or manual level resumes

#### Scenario: Protection never reduces ventilation

- **WHEN** the active schedule or manual level is already higher than the protection level
- **THEN** moisture protection leaves it unchanged (it only raises, never lowers)

#### Scenario: Composition with extreme-heat mode

- **WHEN** both moisture protection and extreme-heat mode are enabled and a protection condition holds
- **THEN** moisture protection takes precedence over the temperature/energy bands

### Requirement: Fall back to temperature-only when humidity is unavailable

The system SHALL fall back to the existing temperature-difference decision whenever the required
humidity readings are missing or stale, so behaviour is never worse than the temperature-only mode.

#### Scenario: Outdoor sensor offline

- **WHEN** the outdoor humidity reading is stale or missing
- **THEN** the mode decides using the supply-vs-exhaust temperature difference as before

#### Scenario: No sensors configured

- **WHEN** no humidity sensors are configured
- **THEN** extreme-heat mode behaves exactly as the temperature-only mode

### Requirement: Aggregate multiple indoor sensors conservatively

When more than one indoor humidity sensor is fresh, the system SHALL aggregate them so the most
at-risk room drives moisture protection (the highest indoor humidity is used).

#### Scenario: One room is humid

- **WHEN** several indoor sensors are fresh and one reports much higher humidity than the others
- **THEN** the moisture-protection decision uses that highest indoor humidity

### Requirement: Record the reason for each decision

Every ventilation level the mode settles on SHALL carry a reason — temperature delta, cooling
assist (energy), moisture protection, or muggy suppression — and level changes SHALL be recorded
with their reason so they can be explained in the UI. The existing minimum decision dwell SHALL
still apply to humidity-driven changes.

#### Scenario: A humidity-driven change is recorded with its reason

- **WHEN** the mode changes the level because of a moisture-protection or cooling-assist condition
- **THEN** the change is recorded with the corresponding reason and respects the decision dwell
