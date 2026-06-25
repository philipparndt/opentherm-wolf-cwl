## Why

Extreme-heat mode currently decides purely on the supply-vs-exhaust **temperature**
difference. But air carries energy as *enthalpy*, not just temperature: humid air holds far
more energy than dry air at the same temperature, so on a muggy day pulling in "cooler" but
wet outside air can actually add energy and make the house feel worse. Two real goals aren't
served today: (1) **don't import latent heat** — judge ventilation by total energy, not just
temperature; and (2) **protect the house from getting too humid** — ventilate to expel indoor
moisture when the outside air is genuinely drier. Both require humidity data, which the CWL
unit doesn't measure but which cheap MQTT sensors (zigbee2mqtt, DHT, etc.) already publish.

And because the decision rules are becoming complex (temperature delta, enthalpy, moisture
protection, sensor freshness), the behaviour must be **clearly visualized** so it's obvious
why the mode chose a given level at any moment.

## What Changes

- Ingest **humidity + temperature (+ optional pressure) from MQTT sensors**: 1–n **indoor**
  sensors and 1 **outdoor** sensor, configured by topic. Parse the `humidity` / `temperature` /
  `pressure` fields from each sensor's JSON payload, store the latest reading per sensor with a
  freshness timestamp. Pressure (when any sensor reports it) is used for accurate enthalpy at the
  local altitude; sensors without it still work.
- Make extreme-heat mode **humidity-aware**: when fresh humidity data is available, judge
  ventilation by **enthalpy** (total air energy) instead of bare temperature, so ventilating assists
  cooling on an energy basis.
- Add **moisture protection** as its **own independently-enableable feature** (separate toggle,
  works year-round regardless of extreme-heat mode): when indoor air is too humid *and* outdoor air
  is drier in absolute terms, turn the CWL on to dehumidify — regardless of the temperature delta —
  and release control once humidity is back in range. Indoor humidity is aggregated as the
  **maximum** across indoor sensors (worst-case room). When both features are on, protection takes
  precedence within the decision.
- Fall back to the existing **temperature-only** behaviour when humidity data is missing or stale,
  so the mode never gets worse than today.
- Add a **decision visualization** to the web UI: a combined temperature + humidity chart over
  the 24 h window, change markers annotated with the **reason** each level change happened
  (temperature / cooling-assist / dehumidify / muggy-suppression), and a live "why" panel showing
  the current inputs (temps, humidities, absolute humidity, enthalpy) and which rule is driving
  the current level. Plus per-sensor online/stale status.
- Configure all of the above from the web UI (sensor topics) and expose current values + the
  active decision reason over `/api/status`.

## Capabilities

### New Capabilities
- `humidity-sensors`: subscribe to configurable MQTT humidity/temperature sensor topics (indoor
  list + outdoor), parse and store the latest per-sensor reading with freshness, and persist the
  topic configuration.
- `humidity-aware-ventilation`: extend extreme-heat mode to decide on enthalpy + absolute humidity
  — enthalpy-based cooling, a moisture-protection override, and graceful fallback to the
  temperature-only rule when sensor data is unavailable or stale. Each decision records a reason.
- `humidity-decision-visualization`: web visualization that makes the (now multi-factor) decision
  legible — combined temperature/humidity history chart, reason-annotated change markers, a live
  decision-explainer panel, and per-sensor status.

### Modified Capabilities
<!-- extreme-heat-mode's spec is not yet archived in openspec/specs/, so the humidity behaviour
     is expressed as new capabilities layered on it rather than as a delta spec. -->

## Impact

- Firmware: `mqtt.rs` (subscribe to sensor topics, parse readings), `app_state.rs` (sensor store
  + freshness, current decision reason), `extreme_heat.rs` (humidity-aware decision + reasons),
  new `psychro.rs` (absolute humidity / enthalpy from T + RH), `config.rs` / `config_manager.rs`
  (sensor topic config, NVS-persisted), `history.rs` (humidity history channels), `webserver.rs`
  (status fields, sensor config endpoints), and `main.rs` wiring.
- Web (`esp32/web/src/`): config UI for sensor topics, the combined chart, the decision-explainer
  panel, and sensor status.
- RAM: adding humidity history channels increases the in-RAM history footprint (see design for the
  budget and mitigation).
