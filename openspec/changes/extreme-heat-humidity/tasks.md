## 1. Psychrometrics

- [x] 1.1 Create `esp32/src/psychro.rs` with pure functions: `saturation_kpa(t)`, `vapor_kpa(t,rh)`, `absolute_humidity(t,rh) -> g/m³`, `enthalpy(t,rh,p_atm_kpa) -> kJ/kg` (pressure-aware)
- [x] 1.2 Unit-test against known psychrometric points (e.g. 27 °C/55 %, 38.7 °C/30.5 %)

## 2. Config & persistence

- [x] 2.1 Add `humidity_inside_topics: Vec<String>`, `humidity_outside_topic: String`, and `humidity_protection_enabled: bool` to `AppConfig` (defaults empty / false)
- [x] 2.2 Persist/load them in `config_manager.rs` as JSON keys (pattern of `save_schedules`); include in backup/restore
- [x] 2.3 Define tunable constants: `SENSOR_STALE_SECS=1800`, `RH_PROTECT=65`, `RH_PROTECT_HIGH=75`, `AH_MARGIN=0.5`, enthalpy bands `H_OFF=2.0`, `H_PARTY=-4.0`

## 3. Sensor ingestion (`mqtt.rs` + `app_state.rs`)

- [x] 3.1 Add a sensor store to `app_state`: indoor map `topic -> HumiditySample{humidity, temperature: Option<f32>, pressure: Option<f32>, updated_ms}`, an outdoor `Option<(String, HumiditySample)>`, and an `ambient_pressure_kpa` (default 101.3)
- [x] 3.2 In `setup_subscriptions` (and a re-subscribe check each tick), subscribe to every configured sensor topic; idempotent so topic edits apply without reboot
- [x] 3.3 In the MQTT receive handler, match configured sensor topics and parse `humidity` (required) + `temperature`/`pressure` (optional), storing the latest sample with a timestamp; update `ambient_pressure_kpa` from any reported pressure; ignore malformed payloads
- [x] 3.4 Add a freshness helper (sample age vs `SENSOR_STALE_SECS`)

## 4. Humidity-aware decision (`extreme_heat.rs`)

- [x] 4.1 Add a `Reason` enum (`TempDelta|CoolingAssist|Dehumidify|MuggySuppression|Manual`) and store current reason in `app_state`; add `reason` to `EhEvent`
- [x] 4.2 Compute fresh indoor aggregate (max `AH`, matching `h`, max `RH`) and outdoor `AH`/`h` from `psychro`, passing `ambient_pressure_kpa` to `enthalpy`; substitute exhaust-inlet temp when a sensor lacks temperature
- [x] 4.3 Implement decision precedence: (1) moisture protection (gated by `humidity_protection_enabled`), (2) enthalpy bands and (3) temperature fallback (both gated by `extreme_heat_enabled`) — preserving the 15-min dwell and manual stickiness
- [x] 4.5 Implement standalone protection as a surgical override: when extreme-heat is off, only raise `requested_vent_level` while a protection condition holds (with RH hysteresis on at `RH_PROTECT`, off below `RH_PROTECT-5`), prevent the scheduler from lowering below it, and release control when it clears; never lower below the schedule/manual level
- [x] 4.4 Tag each applied change with its reason and record it in `eh_events`; set the publish flag so the retained snapshot updates

## 5. History + persistence for humidity

- [x] 5.1 Make `Channel` length-agnostic (use `self.slots.len()` instead of the global `HISTORY_SLOTS`); keep temperature channels at 1440
- [x] 5.2 Add indoor/outdoor humidity channels at ~288 slots (5-min); sample them from the aggregated readings
- [x] 5.3 `/api/history` includes the humidity series + per-event reason; the retained extreme-heat snapshot carries the per-event reason. _(Humidity history is NOT in the retained MQTT snapshot — deferred to keep it within one MQTT buffer / avoid OOM; humidity history rebuilds live after reboot.)_

## 6. API + web visualization

- [x] 6.1 Extend `/api/status` with per-sensor readings (humidity/temperature/pressure) + fresh/stale, the current ambient pressure, the current decision reason, and the derived `AH`/`h` values
- [x] 6.2 Add web config UI (Settings) to edit the indoor topic list + outdoor topic and a separate **Moisture Protection** enable toggle (independent of the extreme-heat toggle), posting to `/api/config`
- [x] 6.3 Combined chart: add indoor/outdoor humidity lines on a right-hand 0–100 % axis alongside the temperature lines, with a legend; degrade gracefully when humidity absent
- [x] 6.4 Annotate change markers with their reason (colour/icon + label)
- [x] 6.5 Add the live decision-explainer panel (current inputs incl. ambient pressure, derived AH/enthalpy, active rule, plain-language sentence, fallback notice)
- [x] 6.6 Add per-sensor status display (last humidity/temperature/pressure + fresh/stale badge)

## 7. Verification

- [x] 7.1 Unit-test the decision precedence: muggy-cooler → no ventilate; warm-drier → ventilate; humid-indoor+drier-outdoor → protect; humid-indoor+humid-outdoor → no protect; stale → temperature fallback; standalone protection (extreme-heat off) raises then releases with hysteresis and never lowers below schedule
- [x] 7.2 Build firmware (`cargo +esp build --release`) and web (`tsc` + `vite`)
- [ ] 7.3 Manual: publish the example sensor payloads (`zigbee2mqtt/og_temp_bad`, `garden/weather/indoor_dht`), confirm readings ingest, the explainer shows the right rule, markers carry reasons, and the chart overlays humidity _(requires a device + broker — flash to verify)_
- [ ] 7.4 Manual: take the outdoor sensor stale and confirm fallback to temperature-only with the explainer noting it _(requires a device)_
