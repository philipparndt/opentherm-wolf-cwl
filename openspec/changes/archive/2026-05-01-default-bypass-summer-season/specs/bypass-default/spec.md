## ADDED Requirements

### Requirement: Default bypass schedule is April 1 to September 15
The default `BypassSchedule` SHALL be enabled with start date April 1 and end date September 15.

#### Scenario: Fresh device with no saved bypass schedule
- **WHEN** no bypass schedule is saved in NVS
- **THEN** the bypass SHALL default to enabled, opening April 1 and closing September 15
