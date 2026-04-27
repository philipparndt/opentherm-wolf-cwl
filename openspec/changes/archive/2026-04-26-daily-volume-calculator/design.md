## Context

The Wolf CWL has configured airflow rates stored in TSP registers:
- TSP 0,1 (U1): Reduced airflow — typically 100 m³/h
- TSP 2,3 (U2): Normal airflow — typically 130 m³/h  
- TSP 4,5 (U3): Party airflow — typically 195 m³/h
- Level 0 (Off): 0 m³/h

These are available in `cwlData.tspValues[]`. The schedules define which level runs at which time for which days. By multiplying airflow rate × duration, we get the total exchanged volume.

## Goals / Non-Goals

**Goals:**
- Show per-day total air volume in m³ on the Schedules tab
- Use actual device TSP airflow rates when available, fall back to defaults
- Compute client-side in the web UI (no firmware computation needed)
- Show breakdown: which segments contribute how much
- Highlight insufficient ventilation days

**Non-Goals:**
- Historical tracking / logging of actual volumes
- Accounting for real-time airflow variations (we use the configured rates)

## Decisions

### 1. Add airflow rates to `/api/status`

Add a new `airflow` object to the status response:
```json
{
  "airflow": {
    "reduced": 100,
    "normal": 130,
    "party": 195
  }
}
```

Read from TSP registers (0,1), (2,3), (4,5) as 16-bit values. Fall back to defaults if TSP not yet scanned.

### 2. Client-side calculation

For each day (Mon-Sun):
1. Build the 1440-minute coverage array (same logic as `WeekTimeline`)
2. Count minutes at each level
3. Volume = sum of (minutes / 60 × airflow_rate) for each level
4. Display as "X m³/day"

### 3. Display layout

A compact table below the weekly timeline:

```
Daily Air Volume
         Volume    Breakdown
Mon      2,340 m³  8h Normal + 8h Reduced + 8h gap
Tue      2,340 m³  8h Normal + 8h Reduced + 8h gap
...
```

Or a simpler per-day bar/number if the breakdown makes it too wide.

### 4. Warning threshold

The German DIN 1946-6 recommends a minimum ventilation rate based on living area. As a simple heuristic: if a day has more than 4 hours with no program (level 0 or uncovered), show a warning indicator. The user can interpret this based on their home size.

We'll highlight days with > 4h uncovered/off in orange.
