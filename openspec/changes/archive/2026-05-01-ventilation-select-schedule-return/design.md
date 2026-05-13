## Context

The rotary encoder on the home page lets users cycle through ventilation levels 0-3 (Off, Reduced, Normal, Party) using `rem_euclid(4)`. Selecting a non-off level sets `schedule_override = true`. The scheduler currently clears override when the active schedule index changes (time slot transition), which causes the unintuitive "snap back" behavior.

The user wants: (a) a way to return to schedule from the encoder UI, and (b) manual selections to stick until explicitly cancelled.

## Goals / Non-Goals

**Goals:**
- Add a 5th selection option "Schedule" to the encoder rotation cycle
- Selecting "Schedule" clears override and returns to schedule-controlled ventilation
- Manual level selections persist across schedule transitions until the user chooses "Schedule"
- Maintain the existing Off → duration sub-stage flow

**Non-Goals:**
- Changing the web UI override behavior (web already has explicit override/clear-override endpoints)
- Adding new schedule configuration options
- Changing MQTT protocol

## Decisions

### 1. Use a sentinel value (4) for the "Schedule" edit option

**Choice**: Extend `edit_vent_level` to accept value 4 as "Schedule". The `VentLevel` enum stays at 0-3 (the actual hardware levels). The value 4 is only meaningful in the edit UI context.

**Rationale**: Keeps the domain enum clean. The display code already uses `edit_vent_level` as a u8 and renders names via a function — we just need to handle 4 in the edit-mode display and in `adjust_edit_value`.

**Alternative considered**: Adding a `Schedule` variant to `VentLevel` — rejected because it would propagate through the OpenTherm protocol code where only 0-3 are valid.

### 2. Cycle order: Off(0) → Reduced(1) → Normal(2) → Party(3) → Schedule(4) → Off(0)

**Choice**: Place "Schedule" after Party, before wrapping back to Off. The cycle uses `rem_euclid(5)` instead of `rem_euclid(4)`.

**Rationale**: Natural position — you scroll through all manual options first, then the "give control back" option. Wrapping past Off in reverse goes to Schedule, which is also intuitive.

### 3. Remove auto-clear of override on schedule transition

**Choice**: Delete the block in `scheduler.rs` (lines 127-134) that clears `schedule_override` when `match_index != self.last_active_index`.

**Rationale**: This was the source of the bug. If the user explicitly chooses a level, it should stay. The only way to return to schedule is now explicit: selecting "Schedule" on the encoder, using the web UI clear-override endpoint, or timed-off expiring.

**Risk**: If a user sets manual and forgets, it stays manual forever. This is acceptable — the display already shows "M" indicator (or will show "Manual" header per the other change), making the state visible.

### 4. "Schedule" selection behavior in exit_edit_mode

**Choice**: When `edit_vent_level == 4` and the user confirms:
- Set `schedule_override = false`
- Set `schedule_active` state triggers the scheduler to apply the current schedule level on next eval
- Cancel any active timed-off

**Rationale**: Cleanly returns to schedule-driven mode. The scheduler's next tick will evaluate and apply the correct level.

### 5. Display text for the new option

**Choice**: Show "Schedule" in the level selection screen (same large centered font as other level names).

## Risks / Trade-offs

- **[Override persists indefinitely]** → Mitigated by visible "M"/"Manual" indicator on home screen; user can always select "Schedule" to return
- **[5 options slightly slower to cycle]** → One extra click from 4 options; acceptable on a rotary encoder
