## Why

After a reboot, the display shows "Connecting..." and then enters standby after 5 minutes. The user has no indication when the network actually comes up. When ethernet connects (or WiFi reconnects), the display should wake up and briefly show the IP address so the user knows the device is online and reachable.

## What Changes

- When the network connects (`ARDUINO_EVENT_ETH_GOT_IP` for ethernet, or WiFi connected state), wake the display for 5 minutes via `displayWake()`.
- Show a temporary IP overlay on the display for 10 seconds after connection, then resume normal page rendering.
- Applies to both ethernet and WiFi builds.

## Capabilities

### New Capabilities

_None_ -- this uses the existing `displayWake()` mechanism and adds a temporary overlay to the display.

### Modified Capabilities

_None_ -- no spec-level behavior changes.

## Impact

- **esp32/src/network.cpp**: Add `displayWake()` calls and set an IP overlay flag when network connects.
- **esp32/src/display.cpp**: Add IP overlay rendering logic with a 10-second timeout.
- **esp32/src/display.h**: Declare the function to trigger the IP overlay from network code.
