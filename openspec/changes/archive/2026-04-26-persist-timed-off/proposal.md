## Why

After a power outage or reboot, all runtime state is lost: timed off timer, manual ventilation level override, and manual bypass override. The user expects the device to resume exactly where it left off — if ventilation was manually set to Party for a gathering, it should stay at Party after a brownout, not silently revert to a schedule.

## What Changes

- Persist the timed off end time as a UTC epoch timestamp in NVS — survives reboot with accurate countdown
- Persist the current manual override state (ventilation level override active, bypass override active) in NVS
- On boot, restore all override states from NVS before the scheduler starts evaluating
- Clear persisted state when overrides are naturally cleared (schedule transition, timed off expiry, user cancellation)

## Capabilities

### New Capabilities

(none — enhancement to existing scheduler persistence)

### Modified Capabilities

(none)

## Impact

- **Modified files**: `scheduler.cpp/.h` (persist/restore timed off + overrides via NVS, epoch-based timer)
- No web UI or MQTT changes — behavior is identical, just survives reboots
