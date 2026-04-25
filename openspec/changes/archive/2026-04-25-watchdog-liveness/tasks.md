## 1. Watchdog Module

- [x] 1.1 Create `watchdog.cpp/.h` — TWDT initialization (30s timeout), `feedWatchdog()` function, reboot reason decoder
- [x] 1.2 Add OpenTherm liveness check — restart if `cwlData.lastResponse` stale for 300s (with 60s startup grace)
- [x] 1.3 Add heap exhaustion check — restart if free heap drops below 20 KB
- [x] 1.4 Add reboot diagnostics — read `esp_reset_reason()` on boot, increment crash counter in NVS for abnormal resets

## 2. Integration

- [x] 2.1 Update `main.cpp` — call `setupWatchdog()` in `setup()`, call `feedWatchdog()` at top of `loop()`
- [x] 2.2 Update `mqtt_client.cpp` — publish health telemetry every 60s (uptime, reboot reason, crash count, free heap, OT response age)
- [x] 2.3 Update `display.cpp` — show reboot reason and crash count on system page
- [x] 2.4 Build verification — compile successful (+3.5KB flash, +24B RAM)
