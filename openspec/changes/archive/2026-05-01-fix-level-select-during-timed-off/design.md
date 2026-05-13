## Context

The flow when selecting a level during timed-off:
1. `exit_edit_mode()` sets `st.cancel_timed_off = true`, `st.requested_vent_level = X`, `st.schedule_override = true`
2. Main loop detects `cancel_timed_off`, calls `sched.cancel_timed_off()`
3. `cancel_timed_off()` currently does: deactivate timer, set `requested_vent_level = 2` (Normal), set `schedule_override = false`

Step 3 overwrites what step 1 set. The race is that the cancel function assumes it should "restore defaults" but the caller already set the desired state.

## Goals / Non-Goals

**Goals:**
- When a user explicitly selects a level while timed-off is active, that level is applied
- `cancel_timed_off()` only deactivates the timer, doesn't touch level/override state

**Non-Goals:**
- Changing the web UI cancel-off behavior (it already works correctly — sets `cancel_timed_off` without changing level)

## Decisions

### 1. Make cancel_timed_off() only cancel the timer

**Choice**: Remove `st.requested_vent_level = 2` and `st.schedule_override = false` from `cancel_timed_off()`. The caller is responsible for setting the desired state before triggering the cancel.

**Rationale**: There are two callers:
- Display `exit_edit_mode` (level 1-3): already sets `requested_vent_level` and `schedule_override = true`
- Display `exit_edit_mode` (level 4 / Schedule): already sets `schedule_override = false`
- Web API cancel endpoint: only sets `cancel_timed_off = true`, doesn't set a level — after cancel, the scheduler's next tick will apply the schedule (since override is already false from timed-off start)

All callers already handle the post-cancel state correctly.

## Risks / Trade-offs

- **[Web cancel without explicit level]** → When the timer originally started, `schedule_override` was set. After cancel with no explicit level, the override remains and the last-set level stays. This is actually correct — the web "Resume Schedule" button separately clears override.
