## Why

The Wolf CWL bypass damper controls whether outdoor air bypasses the heat exchanger. When open, cool outdoor air is fed directly into the house — this is **summer/cooling mode**. When closed, air passes through the heat exchanger for heat recovery — **winter mode**. Currently the bypass is a simple on/off toggle via MQTT or encoder. Users need date-based scheduling so the bypass automatically opens in the warm months and closes in the cold months, without relying on external automations. The terminology "bypass" is confusing — the UI should clearly present this as "Summer Mode (Cooling)".

## What Changes

- Add a date-range-based bypass schedule: configure a start date (day+month) and end date (day+month) defining the summer/cooling season when the bypass is open
- Use clear labeling throughout the UI: "Summer Mode" instead of raw "bypass" — tooltip/help text explains the heat exchanger bypass mechanism
- Persist the bypass schedule in NVS
- Evaluate the schedule daily against the current date
- Allow manual override via MQTT/encoder (same pattern as the ventilation schedule)
- Publish bypass schedule state via MQTT
- Show bypass mode clearly on the OLED display

## Capabilities

### New Capabilities
- `bypass-schedule`: Date-range-based bypass (summer/cooling mode) scheduling with clear UX labeling, NVS persistence, manual override, MQTT integration, and web UI configuration

### Modified Capabilities

(none)

## Impact

- **New logic in**: `scheduler.cpp/.h` (or alongside ventilation schedule if implemented together)
- **Modified files**: `webserver.cpp` (bypass schedule API), `mqtt_client.cpp` (bypass schedule state), `display.cpp` (summer mode indicator), `config_manager.cpp/.h` (bypass schedule struct)
- **Web UI**: Bypass/summer mode section in settings or schedules tab
- **NVS**: New keys for bypass schedule (start month/day, end month/day, enabled)
