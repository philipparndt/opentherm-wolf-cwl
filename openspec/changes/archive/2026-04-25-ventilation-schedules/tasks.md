## 1. Scheduler Module

- [x] 1.1 Create `scheduler.cpp/.h` — ScheduleEntry struct, schedule array, `setupScheduler()` to load from NVS
- [x] 1.2 Implement schedule evaluation — `schedulerLoop()` called once per minute, first-match logic, midnight-spanning support
- [x] 1.3 Implement manual override — `setVentilationManualOverride()`, cleared on schedule transition (track `lastActiveScheduleIndex`)
- [x] 1.4 Implement NVS persistence — `loadSchedules()` / `saveSchedules()` with `sched_N_*` keys

## 2. API & Backend Integration

- [x] 2.1 Add schedule API endpoints in `webserver.cpp` — GET/POST `/api/schedules` with JSON serialization
- [x] 2.2 Update `main.cpp` — call `setupScheduler()` in setup, `schedulerLoop()` in loop
- [x] 2.3 Update `mqtt_client.cpp` — publish `schedule/active`, `schedule/entry_name`, `schedule/override` in sensor data cycle
- [x] 2.4 Update MQTT/encoder/web level-set handlers — call `setVentilationManualOverride()` when user changes level

## 3. Display

- [x] 3.1 Update OLED home page — show "S" when schedule active, "M" when manual override, nothing when no schedule

## 4. Web UI

- [x] 4.1 Add Schedules tab in `App.tsx` — list of schedule cards with add/delete, time inputs, level buttons, day checkboxes
- [x] 4.2 Build verification — firmware SUCCESS (61.2% flash), web UI SUCCESS (25KB JS)
