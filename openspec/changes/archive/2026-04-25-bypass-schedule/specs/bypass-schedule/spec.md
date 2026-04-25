## ADDED Requirements

### Requirement: Bypass date-range schedule
The firmware SHALL support a single date-range bypass schedule with a start date (day+month) and end date (day+month) defining the summer/cooling season when the bypass damper is open.

#### Scenario: Summer season active
- **WHEN** the bypass schedule is enabled with range Apr 15 - Sep 30, and the current date is Jul 10
- **THEN** the bypass is commanded open (summer/cooling mode)

#### Scenario: Winter season
- **WHEN** the bypass schedule is enabled with range Apr 15 - Sep 30, and the current date is Dec 5
- **THEN** the bypass is commanded closed (winter/heat recovery mode)

#### Scenario: Start date boundary
- **WHEN** the current date is exactly the start date (Apr 15)
- **THEN** the bypass opens (summer mode begins)

#### Scenario: After end date
- **WHEN** the current date is Oct 1 (one day after end date Sep 30)
- **THEN** the bypass is closed (winter mode)

#### Scenario: NTP not synced
- **WHEN** NTP time has not been synced
- **THEN** the bypass schedule does not evaluate and the manual bypass state is used

### Requirement: Bypass schedule persistence
The firmware SHALL persist the bypass schedule in NVS.

#### Scenario: Schedule survives reboot
- **WHEN** the ESP32 restarts
- **THEN** the bypass schedule configuration is restored from NVS

### Requirement: Manual bypass override
The firmware SHALL allow manual bypass override via MQTT, encoder, or web API. The override is cleared when the season transitions.

#### Scenario: Manual override during summer
- **WHEN** the bypass schedule has the bypass open (summer) and the user commands bypass closed via MQTT
- **THEN** the bypass closes and remains closed until the next season transition

#### Scenario: Override cleared on season change
- **WHEN** manual override is active and the date crosses from summer to winter (or vice versa)
- **THEN** the override is cleared and the schedule resumes control

### Requirement: Clear summer mode labeling in web UI
The web UI SHALL present bypass control as "Summer Mode (Cooling)" with help text explaining the heat exchanger bypass mechanism.

#### Scenario: Settings display
- **WHEN** the user views the bypass/summer mode section in the web UI
- **THEN** the label reads "Summer Mode" with subtitle "Free cooling — outdoor air bypasses heat exchanger"

#### Scenario: Schedule display
- **WHEN** a bypass schedule is configured
- **THEN** the web UI shows the date range (e.g., "Apr 15 - Sep 30") and current state ("Active" or "Inactive")

### Requirement: Bypass schedule API
The firmware SHALL expose API endpoints for bypass schedule configuration.

#### Scenario: GET bypass schedule
- **WHEN** a GET request is made to `/api/bypass-schedule`
- **THEN** the current bypass schedule is returned as JSON with fields: enabled, startDay, startMonth, endDay, endMonth

#### Scenario: POST bypass schedule
- **WHEN** a POST request with valid schedule JSON is made to `/api/bypass-schedule`
- **THEN** the schedule is saved to NVS and applied immediately

### Requirement: Bypass MQTT publishing
The firmware SHALL publish the bypass/summer mode state via MQTT.

#### Scenario: Bypass state published
- **WHEN** the regular MQTT publish cycle runs
- **THEN** `{topic}/bypass/mode` ("summer" or "winter"), `{topic}/bypass/schedule_active`, and `{topic}/bypass/override` are published

### Requirement: OLED summer/winter mode display
The OLED home page SHALL display the bypass state using clear summer/winter labeling.

#### Scenario: Summer mode on OLED
- **WHEN** the bypass is open (scheduled or manual)
- **THEN** the home page shows "Summer Mode" instead of "Bypass: Open"

#### Scenario: Winter mode on OLED
- **WHEN** the bypass is closed
- **THEN** the home page shows "Winter Mode" instead of "Bypass: Closed"
