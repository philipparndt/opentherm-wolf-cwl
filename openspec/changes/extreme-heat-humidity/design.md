## Context

Extreme-heat mode (`extreme_heat.rs`) decides a ventilation level once a minute from the
supply-vs-exhaust **temperature** delta (ID 80 vs ID 82), with a 15-minute dwell and a
change-event log (`eh_events`) that the web graph renders as markers. The CWL unit has no usable
humidity sensor, but the home already has MQTT humidity sensors (zigbee2mqtt, DHT) that publish
JSON like `{"humidity":54.88,"temperature":27.01,…}`. `mqtt.rs` already runs the MQTT client,
subscribes to topics, and dispatches received messages; `history.rs` keeps RAM-only 1440-bucket
(1-min) channels now persisted via retained MQTT. We want the decision to use **energy
(enthalpy)** and **moisture**, not just temperature, and we want the resulting (more complex)
behaviour to be **clearly visualized**.

## Goals / Non-Goals

**Goals:**
- Ingest 1–n indoor + 1 outdoor humidity/temperature sensors over MQTT, with freshness.
- Judge ventilation by enthalpy when humidity is known; add a moisture-protection override.
- Gracefully fall back to today's temperature-only rule when data is missing/stale.
- Make the decision legible: combined temp/humidity chart, reason-annotated markers, a live
  "why now" explainer, and per-sensor status.

**Non-Goals:**
- No year-round standalone dehumidification mode (this lives inside extreme-heat mode for now).
- No new physical sensors on the CWL; humidity comes only from MQTT.
- No control of anything but the existing ventilation level (no separate damper logic).
- Sensor JSON field names are fixed to `humidity` / `temperature` (both example sensors use them).

## Decisions

### Psychrometrics (`psychro.rs`)
Compute from temperature `T` (°C), relative humidity `RH` (%), and ambient pressure `p_atm` (kPa):
- Saturation vapour pressure (Magnus): `p_sat = 0.61094·exp(17.625·T/(T+243.04))` kPa.
- Vapour pressure `p_v = RH/100 · p_sat`.
- **Absolute humidity** `AH = 2.1674 · p_v·1000 / (T+273.15)` g/m³ — the moisture metric for
  protection (does ventilating actually remove water?). Independent of `p_atm`.
- Humidity ratio `W = 0.622·p_v/(p_atm − p_v)`; **specific enthalpy** `h = 1.006·T + W·(2501 +
  1.86·T)` kJ/kg — the energy metric for cooling (humid air = higher `h`). `W` (and thus `h`)
  **depend on `p_atm`**, which matters at altitude: the example indoor sensor reports 960.9 hPa, not
  sea-level 1013 — ignoring it would bias the humidity ratio by ~5 %.
- `enthalpy(t, rh, p_atm)` takes pressure explicitly; `p_atm` comes from the tracked ambient
  pressure (below). Pure functions, unit-tested against known points.

### Sensor ingestion & storage
- Config gains `humidity_inside_topics: Vec<String>` and `humidity_outside_topic: String`,
  persisted to NVS as JSON (same pattern as schedules — low churn).
- `mqtt.rs` subscribes to each configured topic; on a matching `Received`, parse `humidity`
  (required) and `temperature` / `pressure` (optional) and store a `HumiditySample { humidity,
  temperature, pressure, updated_ms }` in `app_state` (a small map keyed by topic for indoor, one
  slot for outdoor).
- **Ambient pressure:** a single `ambient_pressure_kpa` is updated from any sensor that reports
  `pressure` (most recent wins; indoor and outdoor barometric pressure are locally near-identical),
  falling back to 101.3 kPa if none ever has. `enthalpy()` is always called with it.
- Topic changes take effect by re-subscribing on the next MQTT tick (idempotent subscribe to any
  newly-configured topic); no reboot required.
- **Freshness:** `SENSOR_STALE_SECS = 1800` (30 min). A sample older than that is ignored as live
  data (battery sensors report slowly; 30 min tolerates a missed cycle).

### Aggregation
- Indoor moisture/energy use the **worst-case** sensor: indoor `AH` = max over fresh indoor
  sensors, indoor `h` = the enthalpy of the sensor giving that max (conservative — drives
  protection when *any* room is humid). Each sensor's own `T`+`RH` pair is used (coherent); if a
  sensor lacks temperature, the exhaust-inlet temp (ID 82) is substituted for its `T`.
- Outdoor uses the single outdoor sensor's `T`+`RH`; its `T` may be cross-checked against the
  supply-inlet temp (ID 80) but the sensor pair is used for `AH`/`h`.

### Two independent toggles
Config gains `humidity_protection_enabled` alongside the existing `extreme_heat_enabled`; both are
NVS-persisted booleans with web toggles. **Moisture protection is independent** — it runs year-round
whenever `humidity_protection_enabled`, even with extreme-heat mode off. The enthalpy cooling logic
runs only under `extreme_heat_enabled`. Either feature evaluating uses the same per-minute tick,
15-min dwell, sensor freshness, and `eh_events` log.

### Decision precedence (humidity fresh) — evaluated each minute, 15-min dwell unchanged
1. **Moisture protection** (highest priority, gated by `humidity_protection_enabled`): if
   `RH_in_max ≥ RH_PROTECT (65%)` **and** `AH_out < AH_in − AH_MARGIN (0.5 g/m³)` → ventilate.
   Level = Party if also lower-energy (`h_out < h_in`) **or** `RH_in_max ≥ RH_PROTECT_HIGH (75%)`,
   else Normal (avoid importing a lot of hot air just to dehumidify). Reason = `Dehumidify`.
   `RH_in` is the **maximum** humidity across fresh indoor sensors (worst-case room).
2. **Energy (enthalpy) bands** (gated by `extreme_heat_enabled`) on `Δh = h_out − h_in`,
   generalizing the temperature bands: `Δh > +2 → Off`, `0 < Δh ≤ +2 → Reduced`,
   `−4 ≤ Δh < 0 → Normal`, `Δh < −4 → Party` (kJ/kg). Reason = `CoolingAssist` when this raises the
   level vs. what temperature alone would, `MuggySuppression` when high outdoor energy holds it down,
   else `CoolingAssist`.
3. **Temperature fallback** (extreme-heat only): humidity stale/missing → existing
   supply-vs-exhaust temperature bands. Reason = `TempDelta`.
Manual user changes keep their existing sticky behaviour (reason = `Manual`). Thresholds ship as
named constants (tunable later), consistent with the temperature-mode constants.

### Standalone moisture protection is a surgical override
When protection acts on its own (extreme-heat off), it must NOT take over the level the way
extreme-heat does. It only **raises** ventilation: while a protection condition holds it forces
`requested_vent_level ≥ protection level` and prevents the scheduler from lowering below it (an
upward analogue of timed-off). When the condition clears — with a small RH hysteresis (on at
`RH_PROTECT`, off below `RH_PROTECT − 5%`) plus the dwell — it releases, and the normal schedule /
manual level resumes. When both features are on, extreme-heat owns the level and protection is just
its top-priority rule (step 1 above), so there is no conflict.

### Reason model
A `Reason` enum (`TempDelta | CoolingAssist | Dehumidify | MuggySuppression | Manual`) is stored
as the current decision reason and attached to each `EhEvent` (the event log gains a `reason`
field). Exposed via `/api/status` and the history events so the UI can annotate and explain.

### History channels for humidity (RAM budget)
Humidity changes slowly, so it does **not** need 1-minute resolution. To visualize humidity over
time cheaply, add **two coarser channels** (indoor, outdoor) at ~5-min resolution. This requires
making `Channel` length-agnostic (it already uses a boxed slice; methods switch from the global
`HISTORY_SLOTS` to `self.slots.len()`), so temperature channels stay at 1440 while humidity
channels use ~288 slots.
- Cost: 2 × 288 × 12 B ≈ **7 KB** (vs ~35 KB if we matched 1-min). Current free heap ≈ 96 KB →
  comfortable. Humidity is added to the retained MQTT snapshot and `/api/history` (timestamped
  points already make this resolution-independent).

### Visualization (web)
- **Combined chart:** existing supply/exhaust temperature lines on the left axis; indoor/outdoor
  humidity lines on a **right axis** (0–100%), drawn from the coarser humidity series. Distinct
  colours + a legend naming each series and its axis.
- **Reason-annotated markers:** each change marker is coloured/iconed by reason (temp / cooling /
  dehumidify / muggy) and labelled with the level, with a hover/legend mapping.
- **Decision-explainer panel** (the key piece for legibility): shows current `T_in/T_out`,
  `RH_in/RH_out`, derived `AH_in/AH_out`, `h_in/h_out`, the active rule, and a plain-language
  sentence ("Outdoor air is drier and lower-energy → ventilating to cool and dehumidify → Party").
  In fallback it states "using temperature only (humidity stale/missing)".
- **Sensor status:** list each configured sensor with last value + fresh/stale badge.

## Risks / Trade-offs

- **Bad/again-humid outdoor air** → protection only fires when `AH_out < AH_in`, so we never import
  moisture; muggy outdoor air suppresses cooling via enthalpy. Mitigated by design.
- **Sensor offline (dead battery)** → freshness window + fallback to temperature-only; explainer
  shows the fallback so it's visible, not silent.
- **Conflicting goals (dry vs. heat)** → protection caps at Normal when outdoor is higher-energy,
  only going Party when it also cools; avoids importing lots of hot air just to dehumidify.
- **RAM** → humidity history kept coarse (~7 KB); if heap ever tightens, fall back to even fewer
  humidity slots. Temperature resolution unchanged.
- **Psychrometric accuracy** → Magnus/standard-pressure approximations are well within sensor
  accuracy; pressure used if the sensor provides it.
- **Chart clutter** → dual axis with up to 4 series; mitigated by clear colours, a legend, and
  keeping humidity series coarse/smooth.

## Migration Plan

Additive and config-gated: with no sensors configured, extreme-heat mode is byte-for-byte today's
temperature behaviour. Ships in a normal firmware + web OTA. Sensor config is entered in the web UI
afterwards; humidity logic activates once a fresh outdoor + at least one indoor reading exist.

## Resolved Decisions

- **Moisture protection is independent** of extreme-heat mode (own toggle, year-round). ✓
- **Indoor aggregation is max-humidity** (worst-case room). ✓
- **Sensor JSON field names are fixed** (`humidity` / `temperature` / `pressure`) — not configurable
  for now. ✓
- **Thresholds ship as named constants** (not web-configurable in v1). The live explainer panel
  surfaces the actual AH/enthalpy/RH values against the thresholds, so they can be observed and a
  constant re-flashed if needed. If one is later promoted to the web UI, it is the **indoor RH
  protection %** (the only human-intuitive knob).

## Open Questions

- None blocking. Future: optionally expose the indoor RH protection % in the web UI once real-world
  use indicates a need.
