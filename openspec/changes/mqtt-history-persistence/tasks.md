## 1. State plumbing

- [x] 1.1 Add to `AppStateInner`: `pending_history_json: Option<Vec<u8>>`, `pending_extreme_heat_json: Option<Vec<u8>>` (raw payloads handed from the MQTT callback to the main loop), initialised to `None`
- [x] 1.2 Add a signal the MQTT thread can act on for publishing: e.g. `mqtt_publish_history: bool` set true when a history bucket rolls over, and `mqtt_publish_extreme_heat: bool` set true when extreme-heat changes level / enabled state

## 2. Serialize / restore temperature history (`history.rs`)

- [x] 2.1 Add `TempHistory::snapshot_json(&self, now_epoch: i64) -> String` producing `{bucketMs, nowEpoch, supply, exhaust, delta}` with per-bucket `[min,max]`/`null`, oldest→newest (reuse the `/api/history` shape)
- [x] 2.2 Add `TempHistory::restore_from_snapshot(&mut self, json: &[u8])` that rebuilds each `Channel`'s `slots` + `head` (oldest→newest, head = newest) from a snapshot; ignore malformed input gracefully

## 3. Publish from MQTT (`mqtt.rs`)

- [x] 3.1 In `setup_subscriptions`, subscribe to `{base}/persist/temp_history` and `{base}/persist/extreme_heat`
- [x] 3.2 Add `publish_history_snapshot()` — serialize via `TempHistory::snapshot_json` and `pub_retained` to `persist/temp_history`
- [x] 3.3 Add `publish_extreme_heat_snapshot()` — build `{enabled,currentLevel,lastChangeEpoch,events[]}` JSON and `pub_retained` to `persist/extreme_heat`
- [x] 3.4 In `MqttManager::update`, consume the `mqtt_publish_history` / `mqtt_publish_extreme_heat` flags and call the matching publisher; also publish both once shortly after first connect / recovery

## 4. Trigger publishing from the producers

- [x] 4.1 In `main.rs`, set `mqtt_publish_history = true` when `temp_history.sample()` returns `true` (bucket rollover)
- [x] 4.2 In `extreme_heat.rs`, set `mqtt_publish_extreme_heat = true` when a level change is applied or the enabled state transitions

## 5. Recovery on boot

- [x] 5.1 In `handle_event`/a receive handler, detect messages on the two `persist/` topics; on the FIRST per topic after boot, copy the raw bytes into the matching `pending_*_json` buffer and set a per-topic `*_recovered` guard; drop later messages on those topics
- [x] 5.2 In the main loop, when `pending_history_json` is `Some`, take it and call `TempHistory::restore_from_snapshot`; mark the display dirty so the graph repaints
- [x] 5.3 In the main loop, when `pending_extreme_heat_json` is `Some`, take it and restore `eh_events`, `eh_current_level`, `eh_last_change_epoch` (do NOT override `config.extreme_heat_enabled` — NVS stays authoritative)
- [x] 5.4 Ensure parsing happens only in the main loop (callback does no `serde_json` work)

## 6. Guarantees & guards

- [x] 6.1 Confirm no flash/NVS writes are added on these paths (publishing/recovery touch only RAM + MQTT)
- [x] 6.2 When MQTT is disabled/not connected, ensure none of the new publishing runs and behaviour is unchanged
- [x] 6.3 Handle malformed / empty retained payloads without panicking (graceful no-op, keep current RAM data)

## 7. Verification

- [x] 7.1 Unit-test `snapshot_json` → `restore_from_snapshot` round-trips bucket min/max and ordering (incl. partial/empty channels)
- [x] 7.2 Build firmware (`cargo +esp build --release --features "wifi,simulate-ot,ot-ext"`)
- [ ] 7.3 Manual: with a broker, let history build, reboot, confirm the graph + extreme-heat markers repopulate from retained MQTT and that the broker shows the `persist/*` retained topics _(requires a device + broker — not runnable in this environment)_
- [ ] 7.4 Manual: with MQTT disabled, confirm history still resets on reboot and no `persist/*` topics are published _(requires a device — same limitation)_
