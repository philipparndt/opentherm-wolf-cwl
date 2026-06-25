## Why

The 24 h temperature history and the extreme-heat mode's decision timeline live only in
RAM, so every reboot or OTA wipes them — the graph comes back blank for ~11 minutes and the
change markers are lost. Writing this churning data to flash would wear the NVS partition,
which we explicitly want to avoid. When MQTT is enabled we already have a durable store: the
broker holds **retained** messages. By publishing this state retained and reading it back on
boot, the device can recover everything into RAM with zero extra flash writes.

## What Changes

- When MQTT is enabled, publish the **temperature history** (supply / exhaust / delta buckets)
  as a **retained** message, refreshed whenever a bucket rolls over.
- When MQTT is enabled, publish the **extreme-heat mode activation state** — enabled flag,
  current decided level, dwell timestamp, and the level-change event log (the graph markers) —
  as a **retained** message, refreshed whenever the mode changes the level or its enabled state.
- On boot (MQTT enabled), **subscribe to these retained topics and restore the data into RAM**
  before/while normal sampling resumes, so the graph and markers survive reboots.
- Recovery is **one-shot per topic**: the device applies the first retained snapshot it receives
  after boot and ignores its own later echoes, so there is no feedback loop and live data is
  never clobbered by a stale snapshot.
- **No new flash/NVS writes** are introduced for this data — persistence is MQTT-only. When MQTT
  is disabled the behaviour is unchanged (history/markers remain RAM-only and reset on reboot).

## Capabilities

### New Capabilities
- `mqtt-state-recovery`: publishing the RAM-only temperature history and extreme-heat
  mode-activation state as retained MQTT messages, and recovering them into RAM on boot,
  without writing to flash.

### Modified Capabilities
<!-- The extreme-heat history/markers gain MQTT-backed durability, but their existing
     spec-level behaviour (what is sampled, how the graph renders) does not change, so no
     delta spec is required. -->

## Impact

- Firmware `mqtt.rs`: new retained publishes (history snapshot, extreme-heat snapshot), new
  subscriptions to the persist topics, and receive-side handling that hands raw payloads to the
  main loop for parsing.
- `app_state.rs`: small "pending recovery" buffers + recovery-done flags so the MQTT callback
  thread does no heavy parsing.
- `history.rs`: a serialize/restore path for the bucket channels.
- `main.rs`: apply pending recovery buffers into `temp_history` / extreme-heat state once at boot.
- No changes to `config_manager.rs` / NVS; no new flash writes.
