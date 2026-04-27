## Context

The encoder edit mode currently cycles through levels 0-3. The scheduler has `scheduleOverride` and `setVentilationManualOverride()` for manual level changes. The ventilation level is set via `requestedVentLevel` in `ot_master.h`.

## Goals / Non-Goals

**Goals:**
- Two-stage encoder flow for Off: press → shows "Off" → rotate to set hours (1-99) → press to confirm
- During timed off: level=0, schedules suspended, OLED shows countdown
- After expiry: level returns to Normal (2), schedules resume
- Cancellable: rotate encoder away from Off in stage 1, or set any level via MQTT/web to cancel early
- MQTT: `set/off_timer` accepts hours, `off_timer/remaining` publishes countdown
- Web UI: show remaining time + cancel button

**Non-Goals:**
- Permanent Off mode (always require a timer)
- Minutes-level granularity (hours is sufficient for ventilation)
- Persisting the timer across reboots (if it reboots, resume normal operation)

## Decisions

### 1. State model

New global state in `scheduler.h`:
```cpp
extern bool timedOffActive;
extern unsigned long timedOffEndTime;   // millis() when it expires
extern uint8_t timedOffHours;           // for display
```

When timed off activates:
- `requestedVentLevel = 0`
- `timedOffActive = true`
- `timedOffEndTime = millis() + hours * 3600000`
- Scheduler skips evaluation while `timedOffActive`

When it expires or is cancelled:
- `timedOffActive = false`
- `requestedVentLevel = 2` (Normal)
- Scheduler resumes (clears `scheduleOverride`)

### 2. Encoder two-stage flow

Current flow: press → edit mode → rotate cycles 0/1/2/3 → press confirms.

New flow:
- Press on home page → edit mode, `editVentLevel` = current level
- Rotate to 0 (Off) → display changes to show "Off: 1h" 
- Continue rotating → adjusts hours: 1, 2, 3, ..., 99
- Rotating back past 1h → back to level 3 (Party), exits the Off duration substage
- Press → confirms timed off with selected duration
- Selecting levels 1/2/3 works as before (immediate, no timer)

### 3. OLED display during timed off

Home page shows:
```
    Ventilation
      Off
    Resumes in 2h 45m
```

The countdown updates every display refresh (500ms).

### 4. MQTT topics

```
wolf-cwl/set/off_timer     → command: hours (1-99), starts timed off
wolf-cwl/off_timer/active  → "1" or "0"
wolf-cwl/off_timer/remaining → minutes remaining (0 when inactive)
```

Setting `set/level` to 1/2/3 while timed off is active cancels the timer.

### 5. Web UI

On the Status tab, when timed off is active, show an alert card:
```
⏸ Ventilation Off — resumes in 2h 45m    [Cancel]
```

Cancel button sends level=2 to cancel the timer.
