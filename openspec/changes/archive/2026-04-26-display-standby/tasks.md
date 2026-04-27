## 1. Implementation

- [x] 1.1 Add standby state + `displayWake()` to `display.cpp/.h` — power save enter/exit, activity tracking, timed-off exception
- [x] 1.2 Update `encoderLoop()` in `encoder.cpp` — call `displayWake()` before processing input, discard on wake
- [x] 1.3 Build verification — SUCCESS
