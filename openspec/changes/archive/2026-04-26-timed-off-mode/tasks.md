## 1. Core State

- [x] 1.1 Add timed off state to `scheduler.h/.cpp` — `timedOffActive`, `timedOffEndTime`, `timedOffHours`, `activateTimedOff()`, `cancelTimedOff()`, expiry check in `schedulerLoop()`

## 2. Encoder & Display

- [x] 2.1 Update encoder edit mode in `display.cpp` — two-stage flow: selecting Off enters duration substage (rotate for hours 1-99, rotate back returns to Party)
- [x] 2.2 Update OLED home page — show countdown "Resumes in Xh Ym" during timed off

## 3. MQTT & Web

- [x] 3.1 Update `mqtt_client.cpp` — subscribe to `set/off_timer`, publish `off_timer/active` and `off_timer/remaining`, cancel timed off when `set/level` receives non-zero value
- [x] 3.2 Update `webserver.cpp` — add `/api/status` timed off fields, add `/api/off_timer/cancel` endpoint
- [x] 3.3 Update web UI Status tab — show timed off alert with countdown and cancel button

## 4. Verification

- [x] 4.1 Build verification — firmware SUCCESS, web UI SUCCESS (27.7KB JS)
