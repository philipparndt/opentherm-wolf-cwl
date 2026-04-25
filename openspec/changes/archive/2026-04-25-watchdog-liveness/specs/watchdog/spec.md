## ADDED Requirements

### Requirement: Hardware watchdog resets on loop hang
The firmware SHALL enable the ESP32 Task Watchdog Timer (TWDT) with a 30-second timeout on the main loop task. If `loop()` does not execute within 30 seconds, the TWDT SHALL force a hardware reset.

#### Scenario: Normal operation
- **WHEN** `loop()` executes normally every ~10ms
- **THEN** the TWDT is fed each iteration and no reset occurs

#### Scenario: Hard hang detected
- **WHEN** `loop()` stops executing for more than 30 seconds (deadlock, crash, infinite loop)
- **THEN** the TWDT triggers a hardware reset of the ESP32

### Requirement: OpenTherm liveness monitor
The firmware SHALL monitor the age of the last successful OpenTherm response. If no response has been received for 5 minutes, the firmware SHALL restart the ESP32.

#### Scenario: OpenTherm communication healthy
- **WHEN** the CWL responds to at least one request per polling cycle
- **THEN** no restart is triggered

#### Scenario: OpenTherm bus dead for 5 minutes
- **WHEN** `cwlData.lastResponse` has not been updated for 300 seconds
- **THEN** the firmware logs the failure and calls `ESP.restart()`

#### Scenario: Startup grace period
- **WHEN** the firmware has just booted and no OpenTherm response has ever been received
- **THEN** the liveness monitor waits 60 seconds before starting to check (allowing probe + first cycle)

### Requirement: Heap exhaustion detection
The firmware SHALL monitor free heap and restart if it drops below 20 KB.

#### Scenario: Heap sufficient
- **WHEN** free heap is above 20 KB
- **THEN** no action is taken

#### Scenario: Heap critically low
- **WHEN** free heap drops below 20 KB
- **THEN** the firmware logs a warning and calls `ESP.restart()`

### Requirement: Reboot diagnostics persistence
The firmware SHALL persist reboot diagnostics in NVS: the reset reason code, a crash counter, and the last clean uptime.

#### Scenario: Abnormal reboot detected
- **WHEN** the ESP32 boots and `esp_reset_reason()` indicates a watchdog, panic, or brownout reset
- **THEN** the crash counter in NVS is incremented

#### Scenario: Clean reboot
- **WHEN** the ESP32 boots after a software restart (OTA, user-initiated, factory reset)
- **THEN** the crash counter is NOT incremented

#### Scenario: Crash count readable
- **WHEN** the crash count is queried (via MQTT, web API, or OLED)
- **THEN** the current crash count since last NVS clear is returned

### Requirement: Health telemetry via MQTT
The firmware SHALL publish health data to MQTT periodically (every 60 seconds).

#### Scenario: Health data published
- **WHEN** 60 seconds have elapsed since the last health publish and MQTT is connected
- **THEN** the firmware publishes to `{topic}/health/uptime`, `{topic}/health/reboot_reason`, `{topic}/health/crash_count`, `{topic}/health/free_heap`, and `{topic}/health/ot_last_response_age`

### Requirement: Reboot reason on OLED system page
The firmware SHALL display the last reboot reason on the OLED system page.

#### Scenario: System page shows reboot info
- **WHEN** the user navigates to the system page on the OLED display
- **THEN** the display shows the reboot reason (e.g., "WDT", "Panic", "SW", "Power") and the crash count
