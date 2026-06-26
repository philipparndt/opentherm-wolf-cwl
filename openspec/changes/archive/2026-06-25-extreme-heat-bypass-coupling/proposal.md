## Why

Extreme Heat mode automatically drives the ventilation *level* from the supply-vs-exhaust
temperature difference, but it leaves the **summer bypass damper** on the calendar-based
bypass schedule (`bypass-schedule`), which simply holds the bypass open for a date range.
That calendar is wrong on a hot afternoon: with outdoor (supply) air hotter than indoor
(exhaust), an open bypass routes that hot air straight in instead of letting the heat-recovery
core pre-cool it with the cool outgoing air. On a Wolf CWL (a sensible recuperator with ~85–90 %
effectiveness) that is the difference between the unit acting as a ~0.5 kW heater and being
nearly thermally neutral.

The mode already computes exactly the signal the damper needs. Coupling the bypass to the same
supply-vs-exhaust temperature signal makes free cooling and coolth-recovery automatic, and
completes the night-purge / day-defence behaviour the level logic already implements.

## What Changes

- While Extreme Heat mode is enabled, the mode SHALL also drive the bypass damper
  (`requested_bypass_open`) from the supply (ID 80) vs exhaust (ID 82) temperatures:
  - supply cooler than exhaust by more than 0.5 °C → **bypass open** (free cooling: let cool
    outdoor air in without re-warming it through the core)
  - supply warmer than exhaust by more than 0.5 °C → **bypass closed** (recover coolth: let the
    core pre-cool the hot intake with the cool exhaust air)
  - within the ±0.5 °C hysteresis band → **hold** the current damper position (no chatter)
- The damper decision is based on **temperature**, not enthalpy: the CWL is a sensible
  recuperator, so the bypass governs sensible heat only. The fan-*level* decision is unchanged
  and remains enthalpy-aware.
- The calendar **bypass schedule yields** while Extreme Heat mode is enabled (mutually exclusive
  ownership of the damper, mirroring how the ventilation schedule already yields). It resumes
  automatically when the mode is disabled.
- Only `requested_bypass_open` is touched; the persisted `config.bypass_open` baseline is left
  alone, so automatic toggling does not wear NVS flash (mirrors the existing scheduler bypass).

## Capabilities

### New Capabilities
- `extreme-heat-bypass`: automatic bypass-damper control driven by the supply-vs-exhaust air
  temperature difference while Extreme Heat mode is enabled, including the ±0.5 °C hysteresis
  hold and ownership over the calendar bypass schedule.

### Modified Capabilities
- `bypass-schedule`: the calendar bypass schedule is suppressed while Extreme Heat mode owns the
  damper, and resumes when the mode is disabled.

## Impact

- Firmware (`esp32/src/extreme_heat.rs`): new `select_bypass()` decision function + a named
  `EH_BYPASS_HYST` constant; the update step now sets `requested_bypass_open` each ready eval.
- Firmware (`esp32/src/scheduler.rs`): the bypass schedule is gated on `!extreme_heat_enabled`.
- No config/NVS schema change (rides on the existing `extreme_heat_enabled` switch); no new
  HTTP endpoint. `requested_bypass_open` already flows to the unit via `ot_master.rs` and to
  MQTT (`bypass/mode`).
