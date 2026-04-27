## Context

The OLED display has a standby mechanism: after 5 minutes of no activity (`STANDBY_TIMEOUT = 300000ms`), it powers off via `u8g2.setPowerSave(1)`. The `displayWake()` function resets `lastActivityTime` and re-enables the display. Currently, only encoder input, web virtual-encoder input, and the timed-off countdown prevent standby.

Remote state changes (MQTT commands, scheduler transitions) modify `requestedVentLevel` or `requestedBypassOpen` without calling `displayWake()`, so the display stays dark even though the system state changed.

## Goals / Non-Goals

**Goals:**
- Wake the display for 5 minutes whenever the ventilation level or bypass mode changes via MQTT or the scheduler.
- Use the existing `displayWake()` function -- no new wake mechanism needed.

**Non-Goals:**
- Changing the standby timeout duration.
- Adding a separate "notification" display mode.
- Waking the display for read-only MQTT queries or OpenTherm polling data changes (temperatures, fault codes).

## Decisions

**Call `displayWake()` at the point of state change, not in the polling loop.**

The wake call is added directly in the MQTT handlers and scheduler evaluation code, right where `requestedVentLevel` or `requestedBypassOpen` is assigned. This keeps the wake tightly coupled to actual changes rather than polling for diffs.

In the scheduler, wake is only triggered when the value actually changes (guard with comparison to previous value) to avoid waking the display every 60-second evaluation cycle.

**No page navigation on wake.**

The display wakes on whatever page it was last showing. Automatically jumping to a specific page would conflict with user expectations if they were browsing another page before standby.

## Risks / Trade-offs

- **Scheduler wakes every 60s if schedule is active**: Mitigated by comparing previous vs. new value before calling `displayWake()`. The display only wakes on actual transitions.
- **Thread safety of `displayWake()` from ISR context**: Not a concern -- MQTT callbacks and scheduler run in the main loop, same as the display update. No ISR involvement.
