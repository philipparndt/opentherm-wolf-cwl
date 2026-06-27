## Why

The humidity-aware decision (`humidity::enthalpy_level`) splits "ventilate to cool" from "hold
down (muggy)" at **exactly Δh = 0** (`Δh = h_out − h_in`), with no deadband:

```rust
} else if dh > 0.0 {        // Reduced  (MuggySuppression)
} else if dh >= H_PARTY {   // Normal   (CoolingAssist)
```

Every other boundary in the system is guarded — the bypass damper (`EH_BYPASS_HYST`), moisture
protection (`RH_HYSTERESIS`), the Off edge (`H_OFF = +2`) and the Party edge (`H_PARTY = −4`) — but
the Reduced↔Normal crossing at zero is bare. Two consecutive live readings demonstrate the problem:

| | Indoor h | Outdoor h | Δh | Rule shown | Level |
|---|---|---|---|---|---|
| 1 | 63.2 | 63.5 | +0.3 | Muggy suppression | Reduced |
| 2 | 63.2 | 63.3 | ≈0 (rounded) | Cooling assist | Normal |

Between the two, the outdoor air actually got **0.1 °C warmer** (31.0 → 31.1 °C) and barely drier,
yet the decision flipped from "hold down" to "ventilate to cool". The true Δh merely crossed zero by
a hair. Two problems compound:

1. **The boundary is resolved finer than the inputs are accurate.** Δh is acted on at ~0.1 kJ/kg,
   while a typical DHT humidity sensor (±2–5 % RH) carries ~±1–2 kJ/kg of enthalpy uncertainty. The
   controller is reacting to noise. The 15-min dwell rate-limits the *fan*, but the displayed reason
   refreshes every 60 s (`extreme_heat.rs` keeps the level, refreshes the reason), so the user sees
   the rule flicker Muggy↔Cooling even while the fan is held; and if Δh sits on the boundary the fan
   can still hunt every dwell window.
2. **"Cooling assist" oversells near Δh = 0.** Equal enthalpy ≈ equal wet-bulb temperature, so
   pulling in 31 °C air to a 27 °C room does not actually cool — it trades humidity for sensible
   heat. Real cooling only exists once outdoor energy is clearly lower (which is exactly why the
   Party edge already keeps a −4 margin).

And when the mode is *deliberately* holding — because Δh is inside the neutral deadband, or because
the dwell timer hasn't elapsed — the UI gives no reason for the non-switch. The user cannot tell
"holding on purpose" from "stuck / broken".

## What Changes

- Add **state-dependent hysteresis around Δh = 0** to `enthalpy_level`: enter cooling
  (Normal/CoolingAssist) only when `Δh < H_COOL_ON`, release back to hold-down
  (Reduced/MuggySuppression) only when `Δh > H_COOL_OFF`, and **hold the current state** in the
  neutral band between them. `Party` (`Δh < H_PARTY`) and `Off` (`Δh > H_OFF`) bands are unchanged.
- Make `CoolingAssist` mean *actual* cooling: the "ventilate to cool" reason is only claimed once
  cooling has been entered past `H_COOL_ON`, never on a marginal sub-band crossing.
- **Surface the hold reason in the decision explainer:** when the level is not switched because the
  energy delta is inside the neutral deadband, or because the dwell timer has not elapsed, the panel
  states that explicitly (which guard is holding, and — for the dwell — how much longer).
- Clarify the explainer's outdoor/indoor figures so the displayed `T / RH / AH / h` no longer reads
  as one inconsistent air mass (RH is the max-RH sensor; AH/h come from the wettest sensor
  re-expressed at the aggregate temperature — they legitimately come from different sensors).
- **Boot robustness & honest level (from live-device findings):** don't derive an air state from an
  untrustworthy fallback temperature, and don't show the derived enthalpy in the explainer until the
  unit is connected — this removes the boot-transient "indoor h ≈ 37.9" glitch. The ventilation
  display shows the unit's **actual** running level (ID 77), not the commanded one (ID 71), since the
  two can differ. The explainer always shows a state (a "waiting for the unit" notice while
  disconnected) and every reason has a plain-language explanation.

## Capabilities

### Modified Capabilities
<!-- humidity-aware-ventilation and humidity-decision-visualization are defined in the still-active
     `extreme-heat-humidity` change, not yet archived to openspec/specs/. Following that change's
     own precedent (layering onto un-archived extreme-heat-mode), the boundary-stability behaviour
     is expressed as ADDED requirements that refine the existing enthalpy bands, rather than as a
     MODIFIED delta against a baseline spec that does not yet exist. -->

- `humidity-aware-ventilation`: add boundary hysteresis so the Reduced↔Normal (hold-down↔cooling)
  decision no longer toggles on noise around equal energy, and tie the `CoolingAssist` reason to
  having actually entered cooling.
- `humidity-decision-visualization`: explain *why a switch is being withheld* (neutral deadband or
  dwell), and present the aggregated air figures without implying a single consistent air mass.

## Impact

- Firmware: `esp32/src/humidity.rs` (`enthalpy_level` gains a `was_cooling` argument and the
  `H_COOL_ON` / `H_COOL_OFF` constants + the neutral-band hold), `esp32/src/extreme_heat.rs` (pass
  the current cooling state into `decide`/`enthalpy_level`, and expose a "hold reason" — deadband vs
  dwell-remaining — alongside the active reason), `esp32/src/app_state.rs` (carry the hold reason /
  dwell-remaining for the status payload).
- API/Web: `/api/status` gains the hold reason + dwell-remaining; `StatusTab.tsx` decision-explainer
  renders the "holding because …" line and the clarified air-figure labels; `translations.ts`.
- Behaviour: no change to the Off/Party bands, the 15-min dwell, manual stickiness, or the
  temperature-only fallback. Tuning constants only widen the neutral region around Δh = 0.
