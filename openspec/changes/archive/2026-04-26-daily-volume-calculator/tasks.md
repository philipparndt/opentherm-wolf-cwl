## 1. Firmware

- [x] 1.1 Add `airflow.reduced`, `airflow.normal`, `airflow.party` to `/api/status` response in `webserver.cpp` — reads from TSP registers (0,1), (2,3), (4,5) with defaults 100/130/195

## 2. Web UI

- [x] 2.1 Update `Status` interface in `api.ts` — add `airflow` field
- [x] 2.2 Create `DailyVolume` component in `App.tsx` — compute per-day volume from schedules + airflow rates, show table with totals, warn on >4h uncovered/off
- [x] 2.3 Add `DailyVolume` to Schedules tab below the timeline
- [x] 2.4 Build verification — firmware SUCCESS, web UI SUCCESS (31KB JS)
