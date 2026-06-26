## Context

Extreme Heat mode (`extreme_heat.rs`) runs once per minute and, while
`config.extreme_heat_enabled` is set, owns `requested_vent_level`, choosing it from the
supply-vs-exhaust signal (enthalpy-aware when fresh humidity sensors are present, temperature
bands otherwise). Relevant existing pieces:

- `app_state.rs`: `requested_bypass_open: bool` is the single damper setpoint. `ot_master.rs`
  writes it to the unit as bit 1 of the ID 70 status flags each poll; `mqtt.rs` publishes it as
  `bypass/mode` (summer/winter). `config.bypass_open` is the NVS-persisted manual baseline.
- `scheduler.rs`: every 60 s, if `bypass_schedule.enabled`, sets `requested_bypass_open` from a
  calendar date range (open for the configured summer months). It already syncs
  `extreme_heat_enabled` into the scheduler and suppresses the *ventilation* schedule while the
  mode is enabled; the *bypass* schedule was not yet suppressed.
- The Wolf CWL is a **sensible** counterflow recuperator (no moisture transfer), so the bypass
  damper only affects how much the core conditions (warms/cools) the incoming air temperature.

## Goals / Non-Goals

**Goals:**
- While Extreme Heat mode is enabled, drive the bypass damper from supply vs exhaust temperature:
  open for free cooling when outdoor is cooler, closed to recover coolth when outdoor is hotter.
- Avoid damper chatter near equality via a small symmetric hysteresis band.
- Make the calendar bypass schedule yield to the mode (single owner of the damper), resuming
  automatically when the mode is disabled.

**Non-Goals:**
- No enthalpy/humidity input to the *damper* decision — the bypass moves sensible heat only, so
  temperature is the correct and sufficient basis. (The fan-level decision stays enthalpy-aware.)
- No new config flag or UI toggle — the behaviour rides on the existing `extreme_heat_enabled`
  switch, the master switch for the summer mode.
- No NVS write per automatic toggle (flash wear) — only `requested_bypass_open` is touched, not
  the persisted `config.bypass_open` baseline. Mirrors the existing scheduler bypass behaviour.
- No new HTTP endpoint or history marker for bypass changes (out of scope; logged only).

## Decisions

### Damper decision (banding + hysteresis)
`select_bypass(supply, exhaust, current_open) -> bool`:
`supply < exhaust − 0.5 → open`, `supply > exhaust + 0.5 → closed`, otherwise **hold**
(`current_open`). The ±0.5 °C band (`EH_BYPASS_HYST`) means a full 1 °C swing is needed to flip
back, which suppresses chatter on a mechanical damper without a separate timer. A pure function,
unit-tested like `select_level`.

### Temperature, not enthalpy
The level logic generalizes to enthalpy because the *fan* moves both sensible and latent energy.
The *bypass* only changes whether the sensible core conditions the air, so it is keyed on
temperature. In the muggy-but-cooler edge case (supply < exhaust, but humid) the mode is already
at its Reduced floor, so opening the damper just admits minimum cool-but-humid air the sensible
core could not have dehumidified anyway — a wash, dominated by the unambiguous hot-day case.

### Ownership vs the calendar schedule
The bypass schedule in `scheduler.rs` is gated on `!extreme_heat_enabled`, mirroring the existing
ventilation-schedule suppression. While the mode is on it is the sole writer of
`requested_bypass_open` (besides a transient manual override, which the mode reverts on its next
eval — consistent with "automatic mode owns the decision"). When the mode is disabled the
scheduler's bypass schedule resumes on its next tick.

### Placement in the update step
The bypass update runs every ready eval, right after the live supply/exhaust temps are read and
*independently of* the fan-level path (baseline capture, manual-override adopt, dwell). It is
therefore gated by the same readiness checks (mode enabled, not timed-off, NTP synced, unit
connected); during timed-off the fans are off so the damper position is moot.

## Risks / Trade-offs

- **Manual bypass is overridden within ≤1 min while the mode is on.** Acceptable: the user opted
  into automatic mode; manual damper control means disabling Extreme Heat. Documented behaviour.
- **1-minute eval cadence + 0.5 °C hysteresis** can lag a fast supply-temperature swing slightly;
  acceptable for a mechanical damper and avoids cycling it.
