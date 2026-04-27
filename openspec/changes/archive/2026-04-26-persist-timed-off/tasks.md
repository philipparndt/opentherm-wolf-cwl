## 1. Implementation

- [x] 1.1 Update `scheduler.h` — change `timedOffEndTime` to `time_t timedOffEndEpoch`
- [x] 1.2 Update `scheduler.cpp` — persist timed off end epoch, schedule override, and bypass override to NVS on manual user action only; restore all three in `setupScheduler()`; epoch-based remaining time; deferred NTP validation after reboot
- [x] 1.3 Build verification — SUCCESS
