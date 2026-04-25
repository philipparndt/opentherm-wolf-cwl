## 1. Bypass Schedule Module

- [x] 1.1 Add `BypassSchedule` struct to `scheduler.h` and NVS load/save in `scheduler.cpp`
- [x] 1.2 Create bypass schedule evaluation logic in `scheduler.cpp` — date-range check, summer/winter determination, manual override with season-transition clearing
- [x] 1.3 Wire bypass schedule into `main.cpp` loop (shares the scheduler's per-minute evaluation)

## 2. API & MQTT

- [x] 2.1 Add `/api/bypass-schedule` GET/POST endpoints in `webserver.cpp`
- [x] 2.2 Update `mqtt_client.cpp` — publish `bypass/mode`, `bypass/schedule_active`, `bypass/override`
- [x] 2.3 Update MQTT bypass-set handler to call `setBypassManualOverride()`

## 3. Display & UI

- [x] 3.1 Update OLED home page — replace "Bypass: Open/Closed" with "Summer" / "Winter"
- [x] 3.2 Add Summer Mode section in web UI — date-range inputs, enable toggle, help text, save with schedules
- [x] 3.3 Build verification — firmware SUCCESS (61.2% flash), web UI SUCCESS (25KB JS)
