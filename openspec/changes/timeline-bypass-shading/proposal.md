## Why

The temperature-history timeline is framed as part of Extreme Heat mode — the chart
component is `ExtremeHeatChart`, and the only context it overlays is extreme-heat
level-change events — even though the underlying history is sampled continuously and is
useful on its own. At the same time, the **bypass damper** is now a first-class cooling
control (it follows the supply/exhaust signal in Extreme Heat mode), but the timeline gives
no indication of *when the bypass was open*, which is exactly the context needed to read the
temperature curves ("did opening the bypass actually pull the supply temperature down?").

## What Changes

- **Decouple the timeline from Extreme Heat mode** so it is a standalone "temperature
  timeline" feature: rename the `ExtremeHeatChart` component to a neutral name
  (`TimelineChart`), and treat the chart as the home of temperature/humidity history that
  renders regardless of whether Extreme Heat is enabled. Extreme-heat level-change markers
  become one *optional overlay* on the timeline rather than the timeline's reason for being.
- **Shade the chart background by bypass state over time.** The timeline background is drawn
  in two slightly different gray tones: the normal/base gray when the bypass is **closed**
  (heat recovery), and a **slightly brighter** gray for spans where the bypass was **open**
  (free cooling). The shading aligns to the same time axis as the curves.
- Record bypass state across the 24 h history window on-device (a small ring buffer of
  bypass open/closed transitions, mirroring the existing level-change event log) and expose
  it to the web UI via the existing `/api/history` payload so the frontend can paint the
  background bands. Persist it across reboots alongside the history snapshot.

## Capabilities

### New Capabilities
- `temperature-timeline`: the standalone temperature/humidity history chart, independent of
  Extreme Heat mode — always available, with extreme-heat level changes as an optional
  overlay rather than a dependency.
- `bypass-timeline-shading`: a record of bypass open/closed state over the history window and
  its rendering as a two-tone gray chart background aligned to the time axis.

### Modified Capabilities
<!-- No firmware specs live in openspec/specs/ today; behaviour is added, not changed there.
     The existing extreme-heat-visualization spec (in the extreme-heat-mode change) keeps its
     level-change-marker requirements; this change only reframes ownership of the chart. -->

## Impact

- Web frontend (`esp32/web/src/`): rename `ExtremeHeatChart.tsx` → `TimelineChart.tsx` and
  update its import in `StatusTab.tsx`; add background-band rendering keyed on bypass state;
  add a legend entry and translations (EN/DE) for the bypass shading.
- Firmware (`esp32/src/`): a bypass-state transition log in `app_state.rs` (epoch + open
  flag, bounded ring buffer), recorded where `requested_bypass_open` is set (extreme-heat,
  scheduler, manual/MQTT paths); serialized into the `/api/history` JSON in `history.rs` /
  `webserver.rs`; included in the retained `persist/temp_history` MQTT snapshot for reboot
  recovery.
- API: `/api/history` JSON gains a `bypass` field (list of `{epoch, open}` transitions). No
  new endpoint. No config/NVS schema change.
