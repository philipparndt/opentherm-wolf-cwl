## Why

Users want to know how much total air volume their ventilation system exchanges per day. This is important for evaluating indoor air quality, energy efficiency, and whether the schedule provides sufficient ventilation. The data is already available — the TSP registers contain the configured airflow rates per level (U1=100 m³/h, U2=130 m³/h, U3=195 m³/h) and the schedules define how many hours each level runs. A simple calculation can show the total daily volume per day of the week.

## What Changes

- Add a "Daily Volume" summary card in the web UI Schedules tab, showing per-day total exchanged air volume in m³
- Compute from the configured airflow rates (from TSP registers or defaults) combined with the schedule durations
- Show a breakdown per schedule segment and the total
- Highlight days with low ventilation (below a recommended threshold)

## Capabilities

### New Capabilities

(none — web UI display enhancement using existing data)

### Modified Capabilities

(none)

## Impact

- **Modified files**: Web UI `App.tsx` (new `DailyVolume` component), `api.ts` (use airflow data from status endpoint)
- **Modified files (firmware)**: `webserver.cpp` — add TSP airflow rates to `/api/status` response
- No scheduler or OpenTherm changes
