# MQTT Topics

All topics are prefixed with the configured **base topic** (Settings → MQTT → *Topic*),
shown below as `<base>`. Unless noted otherwise every published message is **retained** and
sent at **QoS 0** (at most once).

Source of truth: `esp32/src/mqtt.rs`.

## Published — ventilation & status

Published every **11 s** (`SENSOR_INTERVAL_MS`).

| Topic | Payload | Meaning |
|---|---|---|
| `<base>/ventilation/level` | `0`–`3` | Current ventilation level (Off / Reduced / Normal / Party) |
| `<base>/ventilation/level_name` | string | Human-readable level name |
| `<base>/ventilation/relative` | `0`–`100` | Relative ventilation % (OpenTherm ID 77) |
| `<base>/temperature/supply` | °C, 1 dp | Supply (outdoor inlet) air temperature (ID 80) |
| `<base>/temperature/exhaust` | °C, 1 dp | Exhaust (indoor) air temperature (ID 82) |
| `<base>/status/filter` | `0` / `1` | Filter dirty / needs replacement |
| `<base>/status/bypass` | `0` / `1` | Bypass / ventilation active |
| `<base>/schedule/active` | `0` / `1` | A schedule is currently driving the level |
| `<base>/schedule/override` | `0` / `1` | Manual override is active over the schedule |
| `<base>/bypass/mode` | `summer` / `winter` | Requested bypass mode |
| `<base>/bypass/schedule_active` | `0` | Reserved (currently constant) |
| `<base>/bypass/override` | `0` | Reserved (currently constant) |
| `<base>/off_timer/active` | `0` / `1` | Timed-off mode active |
| `<base>/off_timer/remaining` | minutes | Time remaining in timed-off mode |

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

Published every **11 s** alongside the status block. These are the aggregated
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

Published every **60 s** (`HEALTH_INTERVAL_MS`).

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

| Topic | Payload | Action |
|---|---|---|
| `<base>/set/level` | `0`–`3` | Set ventilation level (manual override) |
| `<base>/set/bypass` | `1`/`true`/`on` or `0`/`false`/`off` | Open / close bypass |
| `<base>/set/filter_reset` | `1` / `true` | Reset the filter-dirty indicator |
| `<base>/set/off_timer` | minutes (`15`–`20160`) | Start timed-off mode for N minutes |

## Subscribed — humidity sensors (external)

User-configured topics (Settings → Humidity Sensors) that are **not** base-prefixed. Each
payload is JSON with any of `humidity` (%), `temperature` (°C), `pressure` (hPa or kPa).
These feed the `climate/*` psychrometrics and the extreme-heat / moisture-protection logic.
