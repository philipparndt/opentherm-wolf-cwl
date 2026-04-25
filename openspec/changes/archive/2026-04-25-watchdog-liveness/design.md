## Context

The ESP32 firmware runs a single-threaded Arduino loop controlling the Wolf CWL 300 via OpenTherm. If the loop hangs, the CWL loses its master entirely — there is no BM remote as backup. The ESP-IDF framework provides a hardware Task Watchdog Timer (TWDT) that can force a hardware reset if a task stops feeding it within a configurable timeout.

The firmware has several failure modes:
1. **Hard hang**: `loop()` stops executing (deadlock, infinite loop, crash) — TWDT handles this
2. **Soft hang**: `loop()` runs but a subsystem is stuck (WiFi driver blocked, OpenTherm not responding, MQTT permanently disconnected) — software liveness checks handle this
3. **Memory leak**: Heap exhaustion over days/weeks — heap monitoring triggers restart

## Goals / Non-Goals

**Goals:**
- Hardware watchdog (TWDT) resets the ESP32 if `loop()` doesn't execute for 30 seconds
- Software liveness monitor detects prolonged OpenTherm communication failure and restarts
- Heap exhaustion detection triggers restart before crash
- Persist reboot reason + crash count in NVS for diagnostics
- Publish health telemetry via MQTT
- Show reboot reason on OLED system page

**Non-Goals:**
- External watchdog hardware (e.g., TPL5010 timer IC) — that's a future hardware revision
- Automatic ventilation failsafe mode in the CWL firmware itself (out of our control)
- Redundant ESP32 controller

## Decisions

### 1. Use ESP-IDF TWDT (Task Watchdog Timer) with 30-second timeout

The TWDT is a hardware timer. If the registered task doesn't call `esp_task_wdt_reset()` within the timeout, the ESP32 performs a hard reset. 30 seconds is generous enough that brief WiFi blocking (~5s) or slow MQTT reconnects don't trigger false resets, but short enough that a real hang is caught promptly.

**Alternative considered:** Software timer with `ESP.restart()` — rejected because a hard hang would prevent the software timer from firing.

### 2. Software liveness: OpenTherm timeout → restart after 5 minutes

If `cwlData.lastResponse` hasn't been updated in 5 minutes (300 seconds), the OpenTherm bus is dead. This could mean the shield is disconnected, the CWL is powered off, or the OpenTherm interrupt is stuck. After 5 minutes, log the failure and restart — the CWL will resume responding to a fresh master.

WiFi and MQTT already have reconnection logic and don't warrant a full restart.

### 3. Heap exhaustion: restart if free heap drops below 20 KB

Below 20 KB, the ESP32 is likely to crash on the next allocation (WiFi/TLS buffers need ~15 KB). A controlled restart is better than an uncontrolled crash.

### 4. Persist reboot diagnostics in NVS

Store in NVS:
- `reboot_reason`: ESP reset reason code from `esp_reset_reason()`
- `crash_count`: Incremented on each non-clean boot (watchdog, panic, brownout)
- `last_clean_uptime`: Uptime in seconds before last clean reboot (OTA, user-initiated)

This lets the user see via MQTT or OLED whether the device has been crash-looping.

### 5. New file: `watchdog.cpp/.h`

Contains all liveness logic in one module:
- `setupWatchdog()`: Initialize TWDT, load crash counter from NVS
- `feedWatchdog()`: Called from `loop()`, resets TWDT + runs soft checks
- `getRebootReasonStr()`: Human-readable reset reason
- `getCrashCount()`: Read from NVS
- `publishHealthData()`: MQTT health telemetry

## Risks / Trade-offs

- **[False restart from slow WiFi]** → 30s TWDT timeout is generous; WiFi reconnect has a 10s interval. The `delay(10)` in loop ensures TWDT is fed even during WiFi issues. Mitigation: TWDT only watches `loop()`, not WiFi specifically.
- **[Restart during OTA]** → OTA uploads can take 15-20 seconds. The `loop()` still runs during OTA (async web server), so TWDT keeps getting fed. No issue.
- **[NVS write wear]** → Crash count only increments on abnormal reboot, not every cycle. NVS wear is negligible.
