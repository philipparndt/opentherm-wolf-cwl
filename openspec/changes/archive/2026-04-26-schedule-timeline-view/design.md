## Context

Schedules have: startHour, startMinute, endHour, endMinute, ventLevel (0-3), activeDays bitmask (bit0=Mon..bit6=Sun). First match wins. Schedules can span midnight (start > end).

## Goals / Non-Goals

**Goals:**
- Pure CSS + HTML timeline (no canvas/SVG) for minimal bundle size
- 7 rows (Mon-Sun), each a horizontal bar from 0:00 to 24:00
- Colored blocks per schedule segment, gaps shown as warning
- Works on mobile (responsive)

**Non-Goals:**
- Interactive/clickable timeline (edit is via the dialog)
- Showing the current time marker

## Decisions

### 1. Rendering approach: CSS grid/flexbox with percentage widths

Each day row is a `div` with `position: relative`. Schedule segments are absolutely positioned children with `left` and `width` as percentages of 24 hours:
- `left = (startHour * 60 + startMinute) / 1440 * 100%`
- `width = duration / 1440 * 100%`

For midnight-spanning: render two segments (start→24:00 and 0:00→end).

### 2. Color scheme

| Level | Color | Label |
|-------|-------|-------|
| 0 (Off) | `var(--accent-red)` with low opacity | Off |
| 1 (Reduced) | `var(--accent-orange)` | Reduced |
| 2 (Normal) | `var(--accent-blue)` | Normal |
| 3 (Party) | `var(--accent-green)` | Party |
| Uncovered | Striped pattern (diagonal lines) | No program |

### 3. Gap detection

For each day, compute coverage by iterating through matching schedules (first match). Any minute not covered by any schedule is a gap. Render gap segments in the warning style.

### 4. Layout

```
          0    6    12   18   24
Mon  ████████░░░░████████████░░░░
Tue  ████████░░░░████████████░░░░
Wed  ████████░░░░████████████░░░░
Thu  ████████░░░░████████████░░░░
Fri  ████████░░░░████████████░░░░
Sat  ██████████████████████████░░
Sun  ██████████████████████████░░

█ = scheduled   ░ = no program (warning)
```

Day labels on the left (3-letter abbreviation), time axis on top.

### 5. Legend

Small legend below the timeline showing color → level mapping and the gap pattern.
