## Context

The firmware already has NTP time sync with CET/CEST timezone, NVS persistence, a Preact web UI with tabs, and MQTT publishing. The ventilation level is controlled via `requestedVentLevel` in `ot_master.h`. The scheduler needs to integrate with the existing control flow: schedule sets the level, but manual changes (MQTT/encoder) temporarily override until the next schedule transition.

## Goals / Non-Goals

**Goals:**
- Up to 8 schedule entries, each with start time, end time, active days, and ventilation level
- Schedules evaluated every minute against local time (CET/CEST, already configured)
- Manual override: any MQTT/encoder/web change overrides the schedule until the next transition point
- Full CRUD via web UI and REST API
- Persist schedules in NVS
- Display schedule state on OLED and publish via MQTT

**Non-Goals:**
- Recurring patterns beyond weekly (no "every 2nd Tuesday")
- Multiple schedules active simultaneously with priority (first match wins)
- Vacation mode or date-based exceptions
- MQTT-based schedule configuration (web UI only)

## Decisions

### 1. Schedule data model

```cpp
#define MAX_SCHEDULES 8

struct ScheduleEntry {
    bool enabled;
    uint8_t startHour;    // 0-23
    uint8_t startMinute;  // 0-59
    uint8_t endHour;      // 0-23
    uint8_t endMinute;    // 0-59
    uint8_t ventLevel;    // 0-3
    uint8_t activeDays;   // Bitmask: bit0=Mon, bit1=Tue, ..., bit6=Sun
};
```

**Rationale:** Compact struct, easy to serialize to NVS and JSON. Bitmask for days is space-efficient and simple to evaluate. 8 entries covers typical use (night, morning, day, evening × weekday/weekend).

### 2. Evaluation logic: first matching schedule wins

Each minute, iterate through enabled schedules. For the current day-of-week and time:
- If current time falls within [startTime, endTime) and the current day bit is set → apply that level
- If a schedule spans midnight (start > end), it matches if time >= start OR time < end
- First matching schedule wins (order matters)
- If no schedule matches → use the configured default level (`appConfig.ventilationLevel`)

**Rationale:** Simple, predictable. The user controls priority by ordering entries.

### 3. Manual override mechanism

When the user changes the level via MQTT, encoder, or web API, set a flag `manualOverride = true` and store `manualOverrideLevel`. The scheduler skips evaluation while `manualOverride` is true. On the next schedule transition (when the matching schedule entry changes), clear the override.

**Implementation:** Track `lastActiveScheduleIndex`. When the scheduler evaluates and finds a different schedule index (or no match) vs. the last one, that's a transition → clear override.

### 4. NVS storage

Store schedules under the existing `wolfcwl` namespace with keys:
- `sched_count`: number of schedules (0-8)
- `sched_N_en`, `sched_N_sh`, `sched_N_sm`, `sched_N_eh`, `sched_N_em`, `sched_N_lv`, `sched_N_dy` for each entry N

**Rationale:** Follows the existing pattern from config_manager (gpio_, mqtrig_ prefixes).

### 5. New file: `scheduler.cpp/.h`

Contains:
- `setupScheduler()`: Load schedules from NVS
- `schedulerLoop()`: Called from main loop, evaluates once per minute
- `getSchedules()` / `setSchedules()`: JSON serialization for web API
- `setManualOverride()`: Called by MQTT/encoder when user changes level
- `isScheduleActive()`: Returns whether a schedule is currently controlling the level

### 6. Web UI: "Schedules" tab

A new tab between Settings and System. Shows a list of schedule entries as cards. Each card has:
- Enable/disable toggle
- Start time (HH:MM input)
- End time (HH:MM input)
- Ventilation level (4 buttons: Off/Reduced/Normal/Party)
- Day checkboxes (Mon-Sun)
- Delete button

Add button at the bottom (up to 8). Save button persists all schedules.

### 7. MQTT topics

```
wolf-cwl/schedule/active       → "1" or "0" (is a schedule currently controlling the level)
wolf-cwl/schedule/entry_name   → description of active schedule (e.g., "Night 22:00-06:00 Reduced")
wolf-cwl/schedule/override     → "1" or "0" (manual override active)
```

Published with the regular sensor data cycle.

## Risks / Trade-offs

- **[NTP not synced]** → If NTP hasn't synced (time < 2024), the scheduler does nothing and falls back to the default level. This is safe.
- **[Schedule overlap]** → First match wins. No validation for overlapping time ranges — the user is responsible for ordering. The web UI could warn, but it's not critical.
- **[NVS space]** → 8 schedules × 7 keys = 56 NVS entries. Well within limits.
