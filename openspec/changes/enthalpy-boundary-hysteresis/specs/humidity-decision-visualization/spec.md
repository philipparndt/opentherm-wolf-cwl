## ADDED Requirements

### Requirement: Explain why a level switch is being withheld

The decision explainer SHALL state why a ventilation-level switch is being withheld whenever the
level the mode would otherwise choose differs from the level it is currently holding, so a
deliberate hold is never indistinguishable from a stuck or faulty controller. The explainer SHALL
distinguish a hold caused by the energy delta being inside the neutral hysteresis band from a hold
caused by the minimum decision dwell not having elapsed. For a dwell hold the explainer SHALL show
the remaining dwell time and the level it would switch to.

#### Scenario: Held by the neutral energy band

- **WHEN** the enthalpy delta is inside the neutral band so the level is intentionally not switched
- **THEN** the explainer states that the decision is held because outdoor and indoor energy are
  within the neutral band (showing the current delta), rather than implying a switch is due

#### Scenario: Held by the dwell timer

- **WHEN** a different level has been decided but the minimum dwell since the last change has not
  elapsed
- **THEN** the explainer states that the switch is pending the dwell, names the level it will switch
  to, and shows the remaining time

#### Scenario: Nothing withheld

- **WHEN** the current level matches the decided level and no hold is in effect
- **THEN** the explainer shows the active rule with no hold notice

### Requirement: Aggregated air figures are not presented as one air mass

The decision explainer SHALL present the aggregated indoor and outdoor figures so it is clear that
relative humidity, absolute humidity and enthalpy may originate from different sensors — relative
humidity from the highest-RH sensor, and absolute humidity / enthalpy from the wettest sensor
re-expressed at the aggregate (lowest) temperature — so the displayed values are not misread as a
single physically consistent air state.

#### Scenario: Figures that do not reconcile by hand are labelled by origin

- **WHEN** the aggregate temperature and relative humidity come from different sensors than the
  absolute humidity and enthalpy, so they would not reconcile if treated as one air mass
- **THEN** the explainer labels or groups the figures by their origin (or notes it) rather than
  presenting temperature / RH / AH / enthalpy as one consistent reading
