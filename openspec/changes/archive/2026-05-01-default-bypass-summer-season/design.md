## Context

`BypassSchedule` currently derives `Default` which gives all zeros (disabled). The schedule is loaded from NVS via `load_bypass_schedule()` which parses JSON — if the JSON is empty/missing, `serde_json::from_str("{}").unwrap_or_default()` returns the Default.

## Goals / Non-Goals

**Goals:**
- New devices get bypass open April 1 – September 15 out of the box
- Existing configured devices are unaffected (NVS has their saved schedule)

**Non-Goals:**
- Making the default configurable at build time
- Changing the bypass schedule UI or API

## Decisions

### 1. Replace derived Default with manual impl

**Choice**: Remove `Default` from the derive macro and implement `Default` manually with `enabled: true, start_day: 1, start_month: 4, end_day: 15, end_month: 9`.

**Rationale**: The derive gives all-zeros which is never a useful schedule. A manual impl lets us set meaningful defaults.

## Risks / Trade-offs

- **[Existing devices with empty NVS bypass key]** → If a device was configured but never set a bypass schedule, it would get the new default on next boot. This is acceptable — the default is a reasonable schedule and can be adjusted via web UI.
