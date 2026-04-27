## Context

The display is updated every 500ms in `updateDisplay()`. The encoder fires rotation/button events in `encoderLoop()` which call `nextPage()`, `prevPage()`, `enterEditMode()`, `exitEditMode()`, `adjustEditValue()`.

## Goals / Non-Goals

**Goals:**
- Turn off display (U8g2 power save) after 5 minutes of no encoder activity
- Keep display on while `timedOffActive` is true (countdown is useful)
- Wake on any encoder input, consuming that input
- Minimal code changes

**Non-Goals:**
- Dimming (OLED is either on or off, no backlight to dim)
- Screen saver animation

## Decisions

### 1. State tracking in `display.cpp`

```cpp
static bool displayStandby = false;
static unsigned long lastActivityTime = 0;
#define STANDBY_TIMEOUT 300000  // 5 minutes
```

`lastActivityTime` is updated by a new `displayWake()` function called from `encoderLoop()`.

### 2. Standby enter/exit

In `updateDisplay()`:
- If `!displayStandby` and `millis() - lastActivityTime > STANDBY_TIMEOUT` and `!timedOffActive`: enter standby via `u8g2.setPowerSave(1)`, set `displayStandby = true`, skip rendering.
- If `displayStandby`: skip rendering entirely (saves I2C traffic too).

`displayWake()`:
- If `displayStandby`: call `u8g2.setPowerSave(0)`, set `displayStandby = false`, update `lastActivityTime`, return `true` (input was consumed).
- If not standby: update `lastActivityTime`, return `false` (input passes through).

### 3. Encoder integration

In `encoderLoop()`, before processing rotation or button press, call `displayWake()`. If it returns `true`, discard the input (return early).
