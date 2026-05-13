## Why

The bypass schedule defaults to disabled, requiring manual configuration via the web UI after setup. Since the typical use case in Central Europe is to open the bypass (free cooling) during the warm season, a sensible default of April 1 to September 15 saves initial configuration effort.

## What Changes

- Change the `Default` implementation for `BypassSchedule` to `enabled: true, start_day: 1, start_month: 4, end_day: 15, end_month: 9`
- This only affects new/unconfigured devices — existing devices already have their bypass schedule saved in NVS

## Capabilities

### New Capabilities

### Modified Capabilities

## Impact

- `esp32/src/scheduler.rs` — `BypassSchedule` default values
