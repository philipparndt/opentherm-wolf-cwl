# MQTT Topics

All topics are prefixed with the configured **base topic** (Settings → MQTT → *Topic*),
shown below as `<base>`. Unless noted otherwise every published message is **retained** and
sent at **QoS 0** (at most once).

Source of truth: `esp32/src/mqtt.rs`.

## Publishing model — change-only

State topics (everything under *command state*, *sensor values*, *climate* and *health*) are
evaluated on their interval but only put on the wire when the **formatted payload differs**
from the last one sent. Since the payload is formatted before the comparison, the printed precision is
also the deadband: `temperature/*` only republishes on a 0.1 °C change, `climate/*/enthalpy`
on 0.01 kJ/kg, and flags like `status/filter` only on an actual flip.

Two things guarantee consumers still converge:

- **Retained delivery** — a subscriber that connects between changes immediately gets the
  current value from the broker, so skipped repeats cost it nothing.
- **Full refresh every 15 min** (`FULL_REFRESH_INTERVAL_MS`) and on **every broker
  reconnect** — the change cache is bypassed and all state topics are republished. This
  restores the retained set after a broker restart that dropped it, and gives time-series
  consumers a guaranteed sample floor for values that sit still for hours.

Net effect: typically **well under 1 msg/s** instead of the constant ~3.2 msg/s a device that
republished everything on every tick produced. The exact rate depends on how twitchy the
measured values are — the remaining traffic is dominated by `temperature/*` and `climate/*`,
so lowering their printed precision in `publish_sensor_data()` is the lever if it is still
too chatty. Command feedback got *faster*, not slower: the command-state block below is
evaluated every second, so a `set/*` command is echoed back within ~1 s.

## Published — command state (requested vs. actual)

Evaluated every **1 s** (`FAST_INTERVAL_MS`), published when changed.

A `set/*` command does **not** take effect immediately: it lands in the device's
state, is written to the unit on the next OpenTherm poll cycle, and is only
confirmed a cycle later — so up to ~20 s can pass before the unit reports the new
level. This block exists so an external app can behave exactly like the web UI,
which shows the requested level right away with a spinner and clears it once the
unit confirms:

1. Send `set/level` → within ~1 s `ventilation/requested` carries your value and
   `ventilation/pending` goes to `1`. Show the new level immediately.
2. When the unit applies it, `ventilation/actual` follows and
   `ventilation/pending` returns to `0`. Clear the spinner.

`requested` also moves when the level is changed from anywhere else — the rotary
encoder, a schedule, extreme-heat mode or the web UI — so it is the single topic
to follow for "what the device wants to run right now".

| Topic | Payload | Meaning |
|---|---|---|
| `<base>/ventilation/requested` | `0`–`3` | **Optimistic** target level — moves the moment a command is accepted |
| `<base>/ventilation/requested_name` | string | Human-readable target level |
| `<base>/ventilation/actual` | `0`–`3` | Level the unit is really running, mapped from ID 77 |
| `<base>/ventilation/pending` | `0` / `1` | `1` while `requested` ≠ `actual` (command not applied yet) |
| `<base>/ventilation/relative` | `0`–`100` | Relative ventilation % (ID 77) — the raw value `actual` comes from |
| `<base>/ventilation/level` | `0`–`3` | The unit's acknowledgement of our ID 71 write. Trails `requested` by up to one poll cycle; kept for compatibility |
| `<base>/ventilation/level_name` | string | Human-readable `level` |
| `<base>/status/connected` | `0` / `1` | OpenTherm link alive. While `0`, `pending` cannot clear |
| `<base>/status/filter` | `0` / `1` | Filter dirty / needs replacement |
| `<base>/status/bypass` | `0` / `1` | Bypass / ventilation active — the **unit-reported** state |
| `<base>/bypass/mode` | `summer` / `winter` | **Requested** bypass mode (optimistic, same role as `ventilation/requested`) |
| `<base>/bypass/pending` | `0` / `1` | `1` while the requested bypass state ≠ `status/bypass` |
| `<base>/schedule/active` | `0` / `1` | A schedule is currently driving the level |
| `<base>/schedule/override` | `0` / `1` | Manual override is active over the schedule |
| `<base>/off_timer/active` | `0` / `1` | Timed-off mode active |
| `<base>/off_timer/remaining` | minutes | Time remaining in timed-off mode |

> If your app only wants one level topic: use `ventilation/requested` for what to
> display and what to compare its own commands against, and ignore the rest.

## Published — sensor values

Evaluated every **11 s** (`SENSOR_INTERVAL_MS`), published when changed.

| Topic | Payload | Meaning |
|---|---|---|
| `<base>/temperature/supply` | °C, 1 dp | Supply (outdoor inlet) air temperature (ID 80) |
| `<base>/temperature/exhaust` | °C, 1 dp | Exhaust (indoor) air temperature (ID 82) |
| `<base>/bypass/schedule_active` | `0` | Reserved (currently constant) |
| `<base>/bypass/override` | `0` | Reserved (currently constant) |

### Conditional (only when the OpenTherm unit reports the value)

| Topic | Payload | Meaning | Condition |
|---|---|---|---|
| `<base>/fan/exhaust_speed` | RPM | Exhaust fan speed (ID 84) | supports ID 84 |
| `<base>/fan/supply_speed` | RPM | Supply fan speed (ID 85) | supports ID 85 |
| `<base>/airflow/current_volume` | m³/h | Current air volume (TSP 52/53) | TSP 52 valid |
| `<base>/temperature/atmospheric` | °C | Atmospheric/outdoor temperature (TSP 55) | TSP 55 valid |
| `<base>/temperature/indoor` | °C | Indoor temperature (TSP 56) | TSP 56 valid |
| `<base>/pressure/input_duct` | Pa | Input duct pressure (TSP 64/65) | TSP 64 valid |
| `<base>/pressure/output_duct` | Pa | Output duct pressure (TSP 66/67) | TSP 66 valid |
| `<base>/status/frost` | code | Frost-protection status (TSP 68) | TSP 68 valid |
| `<base>/status/bypass_position` | code | Bypass flap position (TSP 54) | TSP 54 valid |

## Published — computed psychrometrics (`climate/*`)

Evaluated every **11 s** alongside the status block, published when changed. These are the aggregated
climate-decision inputs (lowest temperature, highest humidity across each side's sensors)
with derived values — the same numbers shown on the status page. The whole set can be
collected with a single `<base>/climate/#` subscription.

The indoor/outdoor blocks are **skipped** when no side has a fresh humidity sensor (the prior
retained values are left in place). `climate/ambient_pressure` is always published.

| Topic | Payload | Meaning |
|---|---|---|
| `<base>/climate/ambient_pressure` | kPa, 2 dp | Ambient pressure used in the calculations |
| `<base>/climate/indoor/temperature` | °C, 1 dp | Aggregated indoor temperature |
| `<base>/climate/indoor/rh` | %, 1 dp | Aggregated indoor relative humidity |
| `<base>/climate/indoor/ah` | g/m³, 2 dp | Indoor absolute humidity |
| `<base>/climate/indoor/enthalpy` | kJ/kg, 2 dp | Indoor specific enthalpy |
| `<base>/climate/outdoor/temperature` | °C, 1 dp | Aggregated outdoor temperature |
| `<base>/climate/outdoor/rh` | %, 1 dp | Aggregated outdoor relative humidity |
| `<base>/climate/outdoor/ah` | g/m³, 2 dp | Outdoor absolute humidity |
| `<base>/climate/outdoor/enthalpy` | kJ/kg, 2 dp | Outdoor specific enthalpy |

> **Telegraf tip:** subscribe to `<base>/climate/#` with `data_format = "value"`,
> `data_type = "float"` and use `topic_parsing` on `<base>/climate/<side>/<field>` to split
> the side and field into tags.

## Published — health

Evaluated every **5 min** (`HEALTH_INTERVAL_MS`), published when changed. `uptime` and
`ot_response_age` change every time by nature; `reboot_reason`, `crash_count` and
`last_panic` are effectively static and only reappear on the 15-minute full refresh.

| Topic | Payload | Meaning |
|---|---|---|
| `<base>/health/uptime` | seconds | Uptime since boot |
| `<base>/health/free_heap` | bytes | Free heap |
| `<base>/health/reboot_reason` | string | Reason for the last reboot |
| `<base>/health/crash_count` | `0` | Reserved (currently constant) |
| `<base>/health/last_panic` | string | Last captured panic message (empty if none) |
| `<base>/health/ot_response_age` | ms | Age of the last OpenTherm response |

## Published — bridge info

Published once on (re)connect.

| Topic | Payload | Meaning |
|---|---|---|
| `<base>/bridge/state` | `online` | Bridge online marker |
| `<base>/bridge/version` | string | Firmware version |
| `<base>/bridge/ip` | string | Device IP address (when known) |

## Published — retained state snapshots

Published only when the underlying data changes (history bucket rollover / extreme-heat
change), so an empty initial snapshot never overwrites a good retained copy. The device also
**subscribes** to these on boot to restore RAM-only state after a reboot.

| Topic | Payload | Meaning |
|---|---|---|
| `<base>/persist/temp_history` | JSON | Downsampled 24 h history (temperature, humidity, enthalpy, bypass) |
| `<base>/persist/extreme_heat` | JSON | Extreme-heat mode state + level-change events |

## Subscribed — commands

| Topic | Payload | Action | Feedback (≤ ~1 s) |
|---|---|---|---|
| `<base>/set/level` | `0`–`3` | Set ventilation level (manual override) | `ventilation/requested`, `ventilation/pending` |
| `<base>/set/bypass` | `1`/`true`/`on` or `0`/`false`/`off` | Open / close bypass | `bypass/mode`, `bypass/pending` |
| `<base>/set/filter_reset` | `1` / `true` | Reset the filter-dirty indicator | `status/filter` (once the unit clears it) |
| `<base>/set/off_timer` | minutes (`15`–`20160`), or `0` to cancel | Start timed-off mode for N minutes; `0` / `off` / `false` / `cancel` ends a running one | `off_timer/active`, `off_timer/remaining` |

Out-of-range or unparseable payloads are ignored — the feedback topics simply do not move,
which is the signal that a command was rejected. Rejected `set/off_timer` payloads are logged
on the serial console; note that anything between `1` and `14` minutes is *not* a cancel and
is rejected, since the accepted range starts at 15 minutes.

## Subscribed — humidity sensors (external)

User-configured topics (Settings → Humidity Sensors) that are **not** base-prefixed. Each
payload is JSON with any of `humidity` (%), `temperature` (°C), `pressure` (hPa or kPa).
These feed the `climate/*` psychrometrics and the extreme-heat / moisture-protection logic.
