## ADDED Requirements

### Requirement: Automatic bypass-damper control from supply-vs-exhaust temperature

While Extreme Heat mode is enabled and both inlet temperatures are valid, the system SHALL drive
the bypass damper (`requested_bypass_open`) from the supply inlet temperature (OpenTherm ID 80)
relative to the exhaust inlet temperature (OpenTherm ID 82). The system SHALL open the bypass
when the supply is cooler than the exhaust by more than 0.5 °C, and SHALL close the bypass when
the supply is warmer than the exhaust by more than 0.5 °C. The decision SHALL be based on
temperature, because the unit is a sensible heat-recovery core and the bypass governs sensible
heat only.

#### Scenario: Outdoor air cooler than indoor — free cooling

- **WHEN** the supply is more than 0.5 °C below the exhaust
- **THEN** the bypass is opened so cool outdoor air enters without being re-warmed by the core

#### Scenario: Outdoor air hotter than indoor — recover coolth

- **WHEN** the supply is more than 0.5 °C above the exhaust
- **THEN** the bypass is closed so the core pre-cools the hot intake with the cool exhaust air

#### Scenario: Temperatures not yet available

- **WHEN** either the supply or exhaust temperature is missing or invalid
- **THEN** the system makes no automatic bypass change and waits for valid readings

### Requirement: Bypass hysteresis hold

The system SHALL hold the current bypass position when the supply and exhaust temperatures are
within a ±0.5 °C band of each other, so that readings hovering near equality do not cycle the
mechanical damper.

#### Scenario: Reading within the band keeps the damper still

- **WHEN** the supply and exhaust temperatures differ by 0.5 °C or less
- **THEN** the bypass keeps whatever position it currently holds and is not toggled

### Requirement: Extreme Heat mode owns the bypass over the calendar schedule

While Extreme Heat mode is enabled it SHALL be the sole automatic owner of the bypass damper,
and the calendar-based bypass schedule SHALL NOT change the damper. When Extreme Heat mode is
disabled, the calendar bypass schedule SHALL resume control.

#### Scenario: Calendar schedule yields while the mode is enabled

- **WHEN** Extreme Heat mode is enabled and a calendar bypass schedule is also enabled
- **THEN** the calendar schedule does not change `requested_bypass_open`
- **AND** only the temperature-driven bypass decision applies

#### Scenario: Calendar schedule resumes when the mode is disabled

- **WHEN** Extreme Heat mode is disabled
- **THEN** the calendar bypass schedule resumes driving the damper on its next evaluation

### Requirement: Automatic bypass changes do not wear persistent storage

Automatic bypass changes made by Extreme Heat mode SHALL update only the runtime requested
bypass state and SHALL NOT write the persisted bypass baseline to NVS.

#### Scenario: Automatic toggle leaves the persisted baseline unchanged

- **WHEN** Extreme Heat mode opens or closes the bypass automatically
- **THEN** `requested_bypass_open` is updated and applied to the unit
- **AND** the persisted `config.bypass_open` baseline is left unchanged
