## Context

Two datasets live only in RAM and reset on every reboot/OTA:

- **Temperature history** (`history.rs` `TempHistory`): 3 channels (`outdoor`=supply inlet,
  `indoor`=exhaust inlet, `delta`), each 128 buckets of `{min,max}`, rolling ~11.25 min/bucket.
- **Extreme-heat activation** (`app_state.rs`): `config.extreme_heat_enabled`, `eh_current_level`,
  `eh_last_change_epoch`, and `eh_events: VecDeque<EhEvent{epoch,level}>` (the graph markers).

`mqtt.rs` already runs on its own thread, publishes most state as **retained** (`pub_retained`,
QoS AtMostOnce, retain=true), subscribes to `{base}/set/*` command topics, and dispatches
received messages through `handle_event` → `handle_command`. The broker's retained store is a
durable, flash-free place to keep the RAM-only data and read it back on boot.

NVS already persists the low-churn bits (`extreme_heat_enabled`, timed-off, schedule override,
level, bypass); those are out of scope here — we only need MQTT for the high-churn RAM-only data.

## Goals / Non-Goals

**Goals:**
- Publish temperature history and extreme-heat activation as retained MQTT messages.
- On boot (MQTT enabled), restore both into RAM from the retained snapshots.
- Zero new flash/NVS writes; unchanged behaviour when MQTT is disabled.

**Non-Goals:**
- No change to what is sampled or how the graph renders.
- No MQTT authority over the `extreme_heat_enabled` flag — NVS stays authoritative for it.
- No attempt to back-fill the exact wall-clock gap between the last publish and reboot; recovered
  buckets are restored in order with "newest = now" (an approximation, same as the live model).
- No persistence of timed-off / schedule state via MQTT (already NVS-backed, low churn).

## Decisions

### Topics and payloads
Two dedicated retained topics under a `persist/` prefix so they're clearly separate from the
live/observability topics and the command topics:

- `{base}/persist/temp_history` — JSON: `{ "bucketMs", "nowEpoch", "supply":[[min,max]|null,…128],
  "exhaust":[…], "delta":[…] }`, oldest→newest. This mirrors the existing `/api/history` shape, so
  the serializer can be shared.
- `{base}/persist/extreme_heat` — JSON: `{ "enabled", "currentLevel", "lastChangeEpoch",
  "events":[{"epoch","level"},…] }`.

Both published with retain=true. Payloads are a few KB; well within MQTT limits.

### Publish cadence
- History: publish on **bucket rollover** (the moment `TempHistory::sample` returns `true`, ~every
  11 min). The snapshot includes the in-progress head bucket, so at most the current partial bucket
  is at risk between a rollover and a reboot — acceptable. Also publish once shortly after a
  successful recovery/first connect so a fresh broker gets an initial snapshot.
- Extreme-heat: publish whenever the mode applies a level change (a new `EhEvent`) or the enabled
  flag changes. These are rare, so cost is negligible.

This is event-driven off existing signals; no new timers churning the broker.

### Recovery path — defer parsing to the main loop
The MQTT receive callback runs on EspMqttClient's event thread with a limited stack; parsing a
multi-KB JSON there is risky. So the callback does **no parsing**: on a message to a `persist/`
topic it copies the raw bytes into a new `app_state` buffer and returns. The main loop drains the
buffer, parses with `serde_json`, and applies:

- `pending_history_json: Option<Vec<u8>>` → parsed into `TempHistory` via a new
  `TempHistory::restore_from_snapshot(...)` that rebuilds each `Channel`'s `slots`/`head`
  (oldest→newest, head at newest).
- `pending_extreme_heat_json: Option<Vec<u8>>` → restores `eh_events`, `eh_current_level`,
  `eh_last_change_epoch` (NOT `enabled` — see below).

### One-shot recovery / no feedback loop
The device subscribes to both `persist/` topics at `setup_subscriptions`. Because it also publishes
to them, the broker echoes its own messages back. Guard with per-topic flags
(`history_recovered`, `extreme_heat_recovered`): the **first** retained payload received per topic
after boot is queued for apply; once applied, later messages on that topic are dropped in the
callback. This prevents re-applying echoes and prevents a stale snapshot from clobbering fresher
live RAM data. If no retained message exists, the flag simply never trips and the data builds from
live samples — no error.

### Source of truth for the enabled flag
`extreme_heat_enabled` is recovered from **NVS** as today. The retained snapshot carries `enabled`
for observability only; recovery restores just the RAM-only decision history and dwell state. This
avoids an NVS-vs-MQTT conflict and keeps a single authority for the toggle.

### Restore vs. live-sample ordering
Recovery is applied in the main loop as soon as the pending buffer appears (typically within a
second or two of MQTT connecting). The history sampler tolerates being pre-populated: restored
buckets sit in the channel and the next `sample()` continues into the head bucket. If a live sample
lands before recovery, the one-shot guard still lets the snapshot restore the older buckets; the
newest (head) may merge — acceptable, no corruption.

## Risks / Trade-offs

- **Echo storm / re-apply** → mitigated by the one-shot per-topic recovery flags.
- **Callback-thread stack exhaustion parsing JSON** → mitigated by deferring parse to the main loop.
- **Snapshot/now misalignment after a long downtime** → recovered buckets are placed newest=now, so
  a long outage shows the old curve shifted to "now". Acceptable for a 24 h trend view; documented.
- **Broker without retained support / wiped retained store** → recovery silently no-ops; history
  rebuilds from live samples (same as today).
- **Partial head bucket lost on reboot** → at most ~11 min of the in-progress bucket; acceptable.
- **MQTT payload size** → full history JSON is a few KB; fine, but keep it compact (arrays of
  `[min,max]`, `null` for empty) rather than verbose objects.

## Migration Plan

Additive: new topics and code paths only. No NVS/partition changes, no breaking changes. Rolls out
with a normal firmware OTA. On first boot after the update the broker has no `persist/` retained
messages yet, so the device starts as before and begins publishing; subsequent reboots recover.

## Open Questions

- Should history also be published on a slow periodic timer (e.g. every few minutes) to capture the
  in-progress bucket, or is rollover-only sufficient? (Design assumes rollover-only.)
- Should the `persist/` topics be cleared (empty retained publish) on factory reset so a reused
  broker doesn't restore stale data onto a fresh device?
- Is a single combined `persist/state` topic preferable to two topics, to make recovery atomic?
