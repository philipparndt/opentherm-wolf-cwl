## Context

The scheduler has several runtime states that are lost on reboot:
- `timedOffActive` + `timedOffEndTime` (millis-based, lost on reboot)
- `scheduleOverride` (manual ventilation level override)
- `bypassOverride` (manual bypass override)

Schedules themselves are already persisted in NVS and re-evaluated on boot — they don't need override flags persisted. The scheduler will naturally recompute the correct schedule state from the current time.

The only states that need persistence are those triggered by **manual user action** — because a manual change is an explicit user intent that shouldn't be silently undone by a reboot.

## Goals / Non-Goals

**Goals:**
- Timed off: persist end time as epoch, restore countdown after reboot
- Manual override flags: persist only when set by user action (MQTT/encoder/web), not by schedule transitions
- Minimize NVS writes — only on manual user actions (rare events), never on periodic schedule evaluation

**Non-Goals:**
- Persisting schedule evaluation state (schedules self-recover from NTP time)
- Persisting the `scheduleActive` flag (recomputed automatically)

## Decisions

### 1. NVS keys (written only on manual action)

| Key | Type | Written when |
|-----|------|-------------|
| `toff_end` | uint32 | `activateTimedOff()` or `cancelTimedOff()` |
| `sched_ovr` | bool | `setVentilationManualOverride()` or when override clears |
| `bp_ovr` | bool | `setBypassManualOverride()` or when override clears |

**Not written by**: `schedulerLoop()` — schedule transitions clear override flags in RAM only. On reboot, the override is gone and the schedule naturally takes over (correct behavior).

### 2. Timed off: epoch-based

Replace `unsigned long timedOffEndTime` (millis) with `time_t timedOffEndEpoch`. On boot, if `toff_end > 0` and `toff_end > time(nullptr)`, restore timed off. If NTP hasn't synced yet, defer to `schedulerLoop()`.

### 3. Restore on boot

In `setupScheduler()`:
1. Read `toff_end` — if > 0, set `timedOffActive = true` (actual expiry check deferred to loop after NTP sync)
2. Read `sched_ovr` — if true, set `scheduleOverride = true` (the persisted `appConfig.ventilationLevel` is the manual level)
3. Read `bp_ovr` — if true, set `bypassOverride = true`

### 4. NVS write frequency

- `activateTimedOff()`: 1 write (rare — user presses encoder)
- `cancelTimedOff()`: 1 write (rare — user action or timer expiry)
- `setVentilationManualOverride()`: 1 write (rare — user changes level)
- `setBypassManualOverride()`: 1 write (rare — user changes bypass)
- Schedule transitions: **0 writes** (override clears in RAM, schedule recomputes on reboot)

Total: a handful of writes per day at most. NVS wear is negligible.
