## Context

The firmware controls a Wolf CWL heat-recovery ventilation unit over OpenTherm and exposes
a Preact web UI plus an OLED display. Relevant existing pieces:

- `cwl_data.rs`: `VentLevel { Off=0, Reduced=1, Normal=2, Party=3 }`; temperature fields
  `supply_inlet_temp` (ID 80) and `exhaust_inlet_temp` (ID 82), decoded as f8.8.
- `app_state.rs`: `AppStateInner` behind `Arc<Mutex<…>>`; `requested_vent_level: u8` is the
  single setpoint that `ot_master.rs` writes to the unit (ID 71). Also holds `config`,
  `temp_history`, and the timed-off / schedule-override flags.
- `scheduler.rs`: evaluates time-based rules once every 60 s and writes `requested_vent_level`.
  Uses NTP epoch seconds (`esp_idf_svc::sys::time()`), guarded by `epoch > 1_700_000_000`.
- `history.rs` (`TempHistory`): 128 rolling buckets (~11.25 min each ≈ 24 h), channels
  `outdoor` (= supply inlet), `indoor` (= exhaust inlet), `delta`. Sampled ~1 Hz from main loop.
- `webserver.rs`: HTTP server with `/api/status` (JSON) and config/schedule endpoints. There is
  **no** history endpoint today; history is only drawn on the OLED.
- Web frontend (`esp32/web/src/`): Preact + TypeScript, built by Vite into `esp32/data/`,
  served from SPIFFS. No charting library is currently used; `/api/status` is polled every 5 s.

The feature adds an automatic control mode on top of this and surfaces it in the web UI.

## Goals / Non-Goals

**Goals:**
- Automatically pick the ventilation level from `delta = supply_inlet − exhaust_inlet` using the
  thresholds in the spec, with a 15-minute minimum dwell between mode-driven changes.
- Make the mode opt-in and persistent, and keep it fully inert when disabled.
- Visualize supply vs exhaust temperatures in the web UI with markers at each mode-driven change.

**Non-Goals:**
- No new humidity/CO₂/forecast inputs — decisions use only the two inlet temperatures.
- No change to how levels are physically written to the unit (`ot_master.rs` / ID 71 unchanged).
- No hysteresis band beyond the explicit thresholds; toggling is controlled by the dwell timer.
- No persistence of the temperature history or event log across reboots (in-RAM, like today's history).

## Decisions

### Where the control loop lives
New module `extreme_heat.rs` exposing a small state machine, evaluated from the main loop (or
the scheduler tick) once per minute — the same cadence as `scheduler.rs`, which is far finer
than a 15-minute dwell. Rationale: keeps the policy isolated and unit-testable, mirrors the
existing `scheduler.rs` pattern, and avoids bloating `ot_master.rs` (which owns transport).
Alternative considered: folding the logic into `scheduler.rs` — rejected to keep the
temperature policy separate from the time-of-day policy.

### Level mapping (banding)
Compute `delta = supply − exhaust` and map to a `VentLevel`:
`delta > 0.5 → Off`, `0 < delta ≤ 0.5 → Reduced`, `−1.0 ≤ delta < 0 → Normal`, `delta < −1.0 → Party`.
The boundary at `delta = 0` resolves to Normal (supply not strictly greater than exhaust).
Keeping the thresholds as named constants makes future tuning trivial.

### Dwell timer
Store `last_change_epoch: i64` and `current_level: u8` in app state. A new selection is applied
only when `now_epoch − last_change_epoch ≥ 900` **and** the selected level differs from the
current one; applying a change resets `last_change_epoch`. Use NTP epoch seconds (same source and
`> 1_700_000_000` validity guard as `scheduler.rs`) so the timer is robust across the ~49-day
millisecond counter wrap. If epoch is not yet valid, make no change.

### Precedence with manual / timed-off
- Timed-off wins: while `timed_off_active`, the mode does not raise the level.
- Manual change: when the user sets a level (UI/encoder/MQTT) while the mode is enabled, treat it
  as the current decision — adopt that level as `current_level` and reset `last_change_epoch`, so
  the mode waits a full 15 minutes before overriding. This makes manual nudges sticky and avoids
  a fight. Alternative considered: manual change disables the mode — rejected as surprising.
- Extreme Heat mode and time-of-day schedules are mutually exclusive in effect: while the mode is
  enabled it owns `requested_vent_level`; schedule evaluation is suppressed for this period.

### Config
Add `extreme_heat_enabled: bool` to `AppConfig` (NVS-persisted alongside existing settings).
Thresholds and the 15-minute dwell ship as constants (not user-configurable in v1) to keep the
UI simple; they are easy to promote to config later.

### Visualization data path
- New endpoint `GET /api/history` returns the `TempHistory` supply and exhaust channels as JSON:
  per-bucket `{min,max}` arrays ordered oldest→newest, plus `bucketMs` and `newestBucketEpoch`
  (or an age offset) so the frontend can place buckets on a time axis.
- New endpoint `GET /api/heat-events` (or an `events` array folded into `/api/history`) returns a
  bounded ring buffer of `{ epoch, level }` records appended whenever the mode changes the level.
  Capacity sized to cover the ~24 h window (e.g. 32–64 events); oldest dropped when full.
- Reuse the existing `temp_history` sampling — no new sampling path. The event ring buffer lives in
  `app_state.rs` and is appended from `extreme_heat.rs` at the moment a change is applied.

### Web rendering
A new Preact component renders an inline **SVG** line chart (supply and exhaust series, distinct
colours via the existing CSS accent variables, labelled temperature axis), with vertical marker
lines/dots at each event time annotated with the selected level name. SVG over canvas: crisp on
HiDPI, trivially styleable with existing CSS vars, no extra dependency, and the dataset is small
(≤128 buckets). The component fetches `/api/history` (+ events) on the existing poll cadence and
re-renders on new data. A toggle for the mode is added to the status UI and posts to `/api/config`.

## Risks / Trade-offs

- **Sensor noise near a threshold** → the 15-minute dwell already damps this; thresholds are
  named constants so they can be widened if real-world data shows flapping at boundaries.
- **NTP not yet synced after boot** → epoch invalid; the mode makes no change until time is valid
  (same guard the scheduler already relies on). Acceptable: the unit holds its current level.
- **Manual-change precedence may surprise users** ("I set Party but 15 min later it dropped") →
  documented behaviour; the dwell makes it predictable, and disabling the mode restores full manual
  control.
- **History/events are RAM-only** → lost on reboot, so the graph starts empty after a restart. Matches
  today's OLED history behaviour; persisting is out of scope for v1.
- **Bundle size** → SVG chart is hand-rolled (no charting lib) to keep `app.js` small.

## Open Questions

- Should the schedule still be allowed to *lower* the level (e.g. night quiet) while Extreme Heat
  mode is enabled, or is the mode strictly exclusive? (Design assumes exclusive while enabled.)
- Are the thresholds (0.5 °C / 1.0 °C) and the 15-minute dwell final, or should they be exposed as
  config tunables in this change rather than a follow-up?
- Should change events also be emitted to MQTT for external logging/automation?
