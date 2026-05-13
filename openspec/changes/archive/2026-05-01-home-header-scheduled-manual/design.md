## Context

The home page in `draw_home()` currently uses `draw_header(d, s.ventilation)` for both the timed-off and normal states. The info line appends " M" or " S" to indicate mode, which is easy to miss.

## Goals / Non-Goals

**Goals:**
- Header shows "Scheduled" when schedule is driving the level
- Header shows "Manual" when user has overridden the schedule
- Header shows "Ventilation" when no schedule is active and no override
- Remove the " M"/" S" suffix from the info line (redundant with header)

**Non-Goals:**
- Changing the timed-off header (keeps showing "Ventilation" or similar)
- Inverted/filled header styling (simple text change is sufficient)

## Decisions

### 1. Header text selection logic

**Choice**:
```
if schedule_override → "Manual"
else if schedule_active → "Scheduled"  
else → "Ventilation"
```

Applied in the `else` branch of `draw_home()` (normal display, not edit mode, not timed-off).

### 2. i18n strings

- EN: `scheduled` = "Scheduled", `manual` = "Manual"
- DE: `scheduled` = "Zeitplan", `manual` = "Manuell"

Both fit comfortably in the header (helvR08, max ~20 chars).

## Risks / Trade-offs

- **[Timed-off header unchanged]** → Timed-off already has its own visual (large "Off" + countdown), so "Ventilation" header is fine there.
