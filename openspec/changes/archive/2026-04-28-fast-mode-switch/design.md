## Context

`exitEditMode(true)` in `display.cpp` handles two cases:

1. **Ventilation level change** (non-off): Sets `requestedVentLevel`, updates `appConfig.ventilationLevel`, calls `saveConfig()`, and calls `setVentilationManualOverride()`.
2. **Bypass mode change**: Sets `requestedBypassOpen`, updates `appConfig.bypassOpen`, calls `saveConfig()`, and calls `setBypassManualOverride()`.

`saveConfig()` writes ~20 NVS keys (WiFi, MQTT, pins, ventilation, bypass, web auth, JWT) — far more than needed. Each NVS write involves flash erase/write cycles.

The MQTT callback (`mqttCallback`) also calls `saveConfig()` for level and bypass changes — same problem.

## Goals / Non-Goals

**Goals:**
- Make ventilation and bypass mode switches feel instant on the display.
- Only persist state that needs to survive reboots.
- Reduce unnecessary NVS flash wear.

**Non-Goals:**
- Changing the full `saveConfig()` function (it's still needed for settings changes).
- Adding async/deferred NVS writes (unnecessary complexity — just don't write when not needed).

## Decisions

**Don't persist non-off ventilation levels to NVS.**

When the user switches to Reduced, Normal, or Party, the level is set in memory (`requestedVentLevel`, `appConfig.ventilationLevel`) but not written to flash. On reboot, the scheduler takes over anyway, or the default from config applies. The `setVentilationManualOverride()` call already persists the override state via the scheduler's own NVS. Only timed-off needs persistence, which `activateTimedOff()` already handles.

**Persist bypass changes with a targeted NVS write.**

Bypass mode (summer/winter) should survive reboots since it's a seasonal setting. Instead of calling the full `saveConfig()`, add a `saveBypassState()` helper that only writes the single `bypass_open` key. This is fast (~2ms for one key vs ~40ms+ for 20 keys).

**Apply the same fix to MQTT callbacks.**

The MQTT callback paths for ventilation level and bypass also call `saveConfig()`. Apply the same logic: no persist for non-off ventilation, targeted persist for bypass.

## Risks / Trade-offs

- **Non-off level lost on unexpected reboot**: If the user manually sets "Party" and the device reboots before the scheduler runs, the level reverts to the stored default. This is acceptable — the scheduler will set the correct level within seconds of boot.
