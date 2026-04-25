## ADDED Requirements

### Requirement: Schedule entry configuration
The firmware SHALL support up to 8 schedule entries, each with a start time (HH:MM), end time (HH:MM), ventilation level (0-3), active days bitmask (Mon-Sun), and enabled flag.

#### Scenario: Create a schedule entry
- **WHEN** the user creates a schedule via the web UI with start=22:00, end=06:00, level=1 (Reduced), days=Mon-Fri
- **THEN** the schedule is saved to NVS and becomes active

#### Scenario: Maximum entries
- **WHEN** 8 schedules already exist
- **THEN** the web UI prevents adding more entries

### Requirement: Schedule evaluation
The firmware SHALL evaluate enabled schedules once per minute against the current local time and day-of-week. The first matching schedule sets the ventilation level.

#### Scenario: Schedule matches current time
- **WHEN** the current time is 23:00 on Wednesday and a schedule exists for 22:00-06:00, Mon-Fri, level=1
- **THEN** the ventilation level is set to 1 (Reduced)

#### Scenario: Schedule spans midnight
- **WHEN** a schedule has start=22:00 and end=06:00
- **THEN** it matches from 22:00 to 23:59 and from 00:00 to 05:59

#### Scenario: No schedule matches
- **WHEN** no enabled schedule matches the current time and day
- **THEN** the configured default ventilation level is used

#### Scenario: NTP not synced
- **WHEN** the system clock has not been synced via NTP
- **THEN** the scheduler does not evaluate and the default level is used

### Requirement: Manual override
The firmware SHALL support manual override: when the user changes the ventilation level via MQTT, encoder, or web API, the schedule is temporarily overridden until the next schedule transition.

#### Scenario: Manual change overrides schedule
- **WHEN** a schedule is active setting level=1 and the user sets level=3 via MQTT
- **THEN** level=3 is applied and the schedule is overridden

#### Scenario: Override cleared on transition
- **WHEN** manual override is active and the active schedule changes (different schedule matches or no schedule matches)
- **THEN** the override is cleared and the new schedule's level (or default) is applied

### Requirement: Schedule persistence
The firmware SHALL persist all schedule entries in NVS. Schedules survive reboots and firmware updates.

#### Scenario: Schedules survive reboot
- **WHEN** the ESP32 restarts
- **THEN** all previously saved schedules are restored and active

### Requirement: Schedule web UI
The firmware SHALL provide a "Schedules" tab in the web UI for creating, editing, and deleting schedule entries.

#### Scenario: Add schedule via web UI
- **WHEN** the user clicks "Add Schedule" and fills in start time, end time, level, and days
- **THEN** the schedule is added to the list and can be saved

#### Scenario: Delete schedule via web UI
- **WHEN** the user clicks delete on a schedule entry and saves
- **THEN** the schedule is removed from NVS

### Requirement: Schedule API endpoints
The firmware SHALL expose REST API endpoints for schedule CRUD operations.

#### Scenario: GET schedules
- **WHEN** a GET request is made to `/api/schedules`
- **THEN** the current schedule configuration is returned as JSON

#### Scenario: POST schedules
- **WHEN** a POST request with schedule JSON is made to `/api/schedules`
- **THEN** the schedules are validated, saved to NVS, and applied

### Requirement: Schedule MQTT publishing
The firmware SHALL publish the current schedule state via MQTT.

#### Scenario: Schedule state published
- **WHEN** the regular MQTT publish cycle runs
- **THEN** `{topic}/schedule/active`, `{topic}/schedule/entry_name`, and `{topic}/schedule/override` are published

### Requirement: Schedule indicator on OLED
The firmware SHALL show a schedule indicator on the OLED home page.

#### Scenario: Schedule active
- **WHEN** a schedule is currently controlling the ventilation level
- **THEN** the home page displays a clock icon or "S" indicator next to the level name

#### Scenario: Manual override active
- **WHEN** manual override is active
- **THEN** the home page displays an "M" indicator instead of "S"
