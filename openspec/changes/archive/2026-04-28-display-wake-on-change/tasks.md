## 1. MQTT Handlers

- [x] 1.1 Add `displayWake()` call in the MQTT ventilation level handler (`mqtt_client.cpp`, after `requestedVentLevel` is set)
- [x] 1.2 Add `displayWake()` call in the MQTT bypass handler (`mqtt_client.cpp`, after `requestedBypassOpen` is set)
- [x] 1.3 Add `#include "display.h"` to `mqtt_client.cpp` if not already present

## 2. Scheduler

- [x] 2.1 Add `displayWake()` call in `schedulerLoop()` when ventilation level changes due to schedule evaluation (guard: only when new level differs from current `requestedVentLevel`)
- [x] 2.2 Add `displayWake()` call in `activateTimedOff()` when timed off is activated
- [x] 2.3 Add `displayWake()` call in `cancelTimedOff()` when timed off is cancelled
- [x] 2.4 Add `displayWake()` call in `schedulerLoop()` when timed off expires and level resumes to normal
- [x] 2.5 Add `displayWake()` call in `schedulerLoop()` when bypass state changes due to schedule evaluation (guard: only on actual transition)
- [x] 2.6 Add `#include "display.h"` to `scheduler.cpp` if not already present

## 3. Verification

- [x] 3.1 Build firmware and verify no compilation errors
