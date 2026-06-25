## Why

During extreme heat the heat-recovery ventilation can make a home *hotter*: when the
incoming fresh (supply) air is warmer than the indoor (exhaust) air, running the fans
pumps heat into the house. Today the user has to manually drop the level on hot
afternoons and raise it again when the outside air cools below the inside, which is
tedious and easy to forget. An automatic "extreme heat" mode that follows the
supply-vs-exhaust temperature difference removes this manual babysitting and keeps the
house as cool as the ventilation physically allows.

## What Changes

- Add an opt-in **Extreme Heat mode** that automatically chooses the ventilation level
  from the difference between supply inlet air (OpenTherm ID 80) and exhaust inlet air
  (OpenTherm ID 82):
  - supply > exhaust + 0.5 °C → **Off**
  - supply > exhaust          → **Reduced** (low)
  - supply < exhaust          → **Normal**
  - supply < exhaust − 1.0 °C → **Party**
- A chosen level is held for a **minimum dwell of 15 minutes** before a new decision can
  change it, so readings near a threshold do not cause rapid toggling.
- The mode is enabled/disabled from the web UI (and persisted in config). While enabled
  it drives `requested_vent_level`; while disabled behaviour is unchanged. It coexists
  with timed-off and manual override per the rules in design.
- **Visualize the mode in the web UI**: a graph showing supply and exhaust air
  temperatures over the rolling history window, with **markers at the points where the
  mode changed the ventilation level** (annotated with the new level).
- Expose the temperature history (already sampled for the OLED) over a new HTTP endpoint
  so the web graph can render it, plus a short log of extreme-heat level-change events.

## Capabilities

### New Capabilities
- `extreme-heat-mode`: automatic ventilation-level control driven by the supply-vs-exhaust
  air temperature difference, including the 15-minute decision dwell, enable/disable +
  persistence, and interaction with manual override / timed-off.
- `extreme-heat-visualization`: web-UI graph of supply and exhaust temperatures with
  change markers, plus the HTTP endpoint(s) that expose temperature history and
  level-change events to the frontend.

### Modified Capabilities
<!-- No existing OpenSpec specs in openspec/specs/; behaviour is added, not changed. -->

## Impact

- Firmware (`esp32/src/`): new control logic (new module, e.g. `extreme_heat.rs`),
  additional fields in `app_state.rs` (enable flag, last decision + timestamp, change-event
  ring buffer), wiring in `main.rs`, and new routes in `webserver.rs`.
- Config (`AppConfig`, NVS-persisted): new `extreme_heat_enabled` setting and any tunables.
- Web frontend (`esp32/web/src/`): new graph component (Preact, client-side rendered),
  API client additions in `api.ts`, and a status/section to toggle the mode.
- `/api/status` JSON gains extreme-heat state; new history/events endpoint(s) added.
