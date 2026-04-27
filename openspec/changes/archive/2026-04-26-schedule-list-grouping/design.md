## Context

Day bitmask: bit0=Mon, bit1=Tue, ..., bit5=Sat, bit6=Sun. Weekdays = 0x1F (bits 0-4), Weekend = 0x60 (bits 5-6), Every day = 0x7F.

## Goals / Non-Goals

**Goals:**
- Smart day label formatting in both web UI and firmware
- Common patterns get concise labels, uncommon patterns fall back to individual days

**Non-Goals:**
- Changing the data model or how days are stored

## Decisions

### 1. Grouping rules (priority order)

| Bitmask | Label |
|---------|-------|
| 0x7F (all 7) | Every day |
| 0x1F (Mon-Fri) | Weekdays |
| 0x60 (Sat-Sun) | Weekend |
| Other | List individual day names, comma-separated |

### 2. Implementation

Replace the `daysStr()` function in the web UI and the day formatting in `getActiveScheduleDescription()` in `scheduler.cpp` with the same logic.
