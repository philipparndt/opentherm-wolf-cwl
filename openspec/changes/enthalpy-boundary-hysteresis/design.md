## Context

`humidity::enthalpy_level` (`esp32/src/humidity.rs`) maps the enthalpy delta `Δh = h_out − h_in`
onto a ventilation level + a cooling flag:

```rust
pub fn enthalpy_level(inp: &HumidityInputs) -> (VentLevel, bool) {
    let dh = inp.outdoor.h - inp.indoor.h;
    if dh > H_OFF { (Off, false) }          // H_OFF  =  2.0
    else if dh > 0.0 { (Reduced, false) }   // <-- bare boundary at exactly 0
    else if dh >= H_PARTY { (Normal, true) }// H_PARTY = -4.0
    else { (Party, true) }
}
```

`extreme_heat::decide` turns the cooling flag into the displayed `Reason` (`CoolingAssist` vs
`MuggySuppression`) and the caller floors the level at `Reduced` (a fully-off CWL leaves stale duct
air). So the *only* boundary that changes the actual fan level between hold-down and active cooling
is the `dh > 0.0` test — and it has no hysteresis and no state memory, unlike every other decision
edge in the codebase (`EH_BYPASS_HYST`, `RH_HYSTERESIS`, and the `±2 / −4` margins on the Off/Party
edges). The dwell (`EH_DWELL_SECS = 15 min`) rate-limits fan *changes* but the reason is refreshed
every eval (60 s), so the displayed rule flickers Muggy↔Cooling on noise, and a Δh parked near zero
still hunts once per dwell window.

Two real readings (indoor h = 63.2 both times; outdoor h 63.5 → 63.3; outdoor T 31.0 → **31.1 °C**)
flipped the decision from MuggySuppression/Reduced to CoolingAssist/Normal even though the outdoor
air got *warmer*. The displayed figures only carry one decimal (0.1 kJ/kg), finer than DHT-class
sensors are accurate (±2–5 % RH ≈ ±1–2 kJ/kg). The control is chasing noise.

## Goals / Non-Goals

**Goals**
- A neutral band around `Δh = 0` within which the Reduced↔Normal (hold-down↔cooling) state is
  *held*, so sensor noise no longer flips the level or the displayed reason.
- `CoolingAssist` is only claimed when cooling has genuinely been entered (Δh clearly negative), not
  on a marginal zero-crossing.
- The explainer states *why a switch is being withheld* — neutral deadband, or dwell not elapsed
  (with remaining time) — so a deliberate hold is distinguishable from a fault.
- The explainer's air figures no longer read as a single inconsistent air mass.

**Non-Goals**
- No change to the Off (`> +2`) or Party (`< −4`) bands, the 15-min dwell, manual stickiness, the
  bypass-damper logic, or the temperature-only fallback.
- No new persisted/NVS state — the cooling state is derived from the current reason, the
  dwell-remaining is computed from the existing `eh_last_change_epoch`.
- No sensor-fusion or accuracy improvements — this is purely about boundary stability and legibility.

## Decisions

### Hysteresis band around Δh = 0

Introduce two constants and a `was_cooling` input, mirroring `protection_level(&inp, was_active)`:

```rust
/// Enter cooling (favour ventilating) only once outdoor air is at least this much
/// lower-energy than indoor (kJ/kg). Above H_COOL_OFF, release back to hold-down.
pub const H_COOL_ON: f32 = -1.0;
pub const H_COOL_OFF: f32 = 0.0;

pub fn enthalpy_level(inp: &HumidityInputs, was_cooling: bool) -> (VentLevel, bool) {
    let dh = inp.outdoor.h - inp.indoor.h;
    if dh > H_OFF { return (VentLevel::Off, false); }
    if dh < H_PARTY { return (VentLevel::Party, true); }
    // Neutral band [H_COOL_ON, H_COOL_OFF]: hold whatever we were doing.
    let cooling = if was_cooling { dh < H_COOL_OFF } else { dh < H_COOL_ON };
    if cooling { (VentLevel::Normal, true) } else { (VentLevel::Reduced, false) }
}
```

- Not cooling → only switch to cooling when `Δh < −1.0`.
- Already cooling → only release when `Δh > 0.0`.
- Between the two it holds, so a Δh wandering across zero by noise changes nothing.

`H_COOL_ON = −1.0` is ~ one sensor-noise unit below zero — small enough that genuine cooling
opportunities are still taken, large enough to clear DHT jitter. Tunable; documented next to the
existing `H_OFF`/`H_PARTY`.

**Why this band and not just rounding Δh:** rounding still flips at the rounded boundary. State-
dependent hysteresis is what the rest of the codebase already uses; this makes the energy decision
consistent with it.

### Deriving `was_cooling` without new state

`extreme_heat::decide` already holds `st.eh_current_reason`. Treat `Reason::CoolingAssist` as the
"currently cooling" state; anything else (MuggySuppression, Dehumidify, TempDelta, Manual) is
not-cooling. No new field, survives the retained-snapshot restore for free (reason is already
restored).

### Hold reason for the explainer

`decide` currently returns `(level, reason)`. Extend the status path so the UI can show *why no
switch happened*. The level is withheld for exactly two reasons, both already computable in
`update()`:

- **Neutral deadband:** `enthalpy_level` returned the same level it’s holding because Δh is inside
  `[H_COOL_ON, H_COOL_OFF]`. Surface a flag like `hold = Deadband` with the current Δh and the band.
- **Dwell:** a *different* level was decided but `now_epoch − eh_last_change_epoch < EH_DWELL_SECS`.
  Surface `hold = Dwell` with `remaining = EH_DWELL_SECS − elapsed` seconds and the level it would
  switch to.

Expose these on `/api/status` (e.g. `holdReason`, `dwellRemainingSecs`, `pendingLevel`) so the
explainer can render "Holding Reduced — outdoor energy within the neutral band (Δh +0.1 kJ/kg)" or
"Holding Reduced — would switch to Normal in 7 min (dwell)". When nothing is held, the fields are
absent/null and the panel shows the active rule as today.

### Air-figure labelling (smaller)

The explainer prints `Outdoor (T / RH / AH / h)` as if one air mass, but `aggregate_side` sets `rh`
to the **max-RH** sensor while `ah`/`h` come from the **wettest** sensor re-expressed at the
aggregate (lowest) temperature — legitimately different sensors, so e.g. 31.1 °C / 24.6 % does not
reconcile with AH 13.5 g/m³ by hand. Relabel/group so this is clear (e.g. show RH as "max RH" and
AH/h as "from wettest sensor @ aggregate T"), or add a one-line note. No logic change.

## Risks / Trade-offs

- **Missed cooling in the −1…0 band.** When Δh is between −1 and 0 and we were not already cooling,
  we now hold Reduced instead of going Normal. That foregoes at most ~1 kJ/kg of marginal cooling
  that is within sensor noise anyway — an acceptable trade for stability. If field data shows it’s
  too conservative, lower `H_COOL_ON` toward −0.5.
- **`was_cooling` from the reason couples decision to display state.** Mitigated: the reason is
  already the single source of truth for the cooling flag and is persisted/restored; no divergence.
- **Tests that call `enthalpy_level(&inp)` must pass the new arg.** Mechanical; covered in tasks.

## Migration / Tuning

Pure firmware + web change; no config or NVS migration. New constants ship with defaults
`H_COOL_ON = −1.0`, `H_COOL_OFF = 0.0`. The boundary near the old `dh > 0.0` behaviour is recovered
by setting both to `0.0`.
