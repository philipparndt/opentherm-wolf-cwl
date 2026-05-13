## Context

The display currently shows `level_name(lang, st.cwl_data.ventilation_level)` — the CWL-confirmed level. The requested level is in `st.requested_vent_level`. After a switch, the CWL typically confirms within 1-2 OpenTherm cycles (~1-2 seconds).

## Goals / Non-Goals

**Goals:**
- Show the target level immediately when a switch is pending
- Use ">" prefix to indicate pending state (e.g. "> Reduced")
- Revert to normal display once confirmed

**Non-Goals:**
- Showing a spinner or animation
- Timeout handling (if CWL never confirms)

## Decisions

### 1. Detection: compare requested vs confirmed

**Choice**: `if st.requested_vent_level != st.cwl_data.ventilation_level` → pending state.

**Rationale**: Simple, no extra state needed. Both values are already in AppStateInner. The comparison is only meaningful when not in timed-off mode (timed-off sets level to 0 which the CWL may report differently).

### 2. Display format with ">" prefix

**Choice**: When pending, show `"> {level_name}"` using `draw_centered`. The ">" prefix fits within the 124px large font width for all level names.

**Rationale**: Short, clear, doesn't require new fonts or icons. The ">" arrow visually suggests "switching to".

### 3. Only apply in normal display branch

**Choice**: Only show the pending indicator in the `else` branch of `draw_home()` (not during edit mode or timed-off). During timed-off, the level is always "Off" and the CWL may report 0 or the pre-off level depending on timing.

## Risks / Trade-offs

- **[Brief flash on startup]** → On boot, requested may differ from confirmed for a moment. Acceptable — it's accurate (the level truly is pending).
