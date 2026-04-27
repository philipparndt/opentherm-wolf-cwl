## Context

The display currently wakes only on encoder/web input or state changes (from the `display-wake-on-change` work). Network connection events happen in `network.cpp` via the `onEthEvent` callback (ethernet) or `networkLoop()` polling (WiFi). The display has no awareness of network events.

The boot screen shows "Connecting..." but once the display enters standby or moves to a normal page, there's no feedback when the network actually comes up.

## Goals / Non-Goals

**Goals:**
- Wake the display when the network connects.
- Show the IP address prominently for 10 seconds, then resume normal rendering.
- Work for both ethernet (`USE_ETHERNET`) and WiFi (`USE_WIFI`) builds.

**Non-Goals:**
- Showing the IP on every reconnect after a brief dropout (only trigger on transitions from disconnected to connected).
- Persisting any state across reboots.
- Changing the system page layout (IP is already shown there permanently).

## Decisions

**Add a `displayShowIP(const char* ip)` function in display.cpp/h.**

This function calls `displayWake()` to wake the display, stores the IP string and records a timestamp. During `updateDisplay()`, if the IP overlay is active (within 10 seconds), render a centered IP screen instead of the current page. After 10 seconds, the overlay clears and normal page rendering resumes. The standby timer reset from `displayWake()` keeps the display on for the full 5 minutes.

**Call `displayShowIP()` from the network event handlers.**

For ethernet: call from `onEthEvent()` on `ARDUINO_EVENT_ETH_GOT_IP`. For WiFi: call from `networkLoop()` when transitioning from disconnected to connected. This keeps the network code as the single source of truth for connection state.

**IP overlay renders on top of page navigation.**

The overlay does not change `currentPage`. When the overlay expires, the user sees whatever page they were on. Encoder input during the overlay calls `displayWake()` (already resets activity time) and clears the overlay so the user can interact immediately.

## Risks / Trade-offs

- **Ethernet event callback context**: `onEthEvent` runs in the WiFi/ETH event task context, not the main loop. Calling `displayWake()` from there touches `lastActivityTime` (an `unsigned long` write, atomic on ESP32) and `displayStandby` (a `bool`). The `displayShowIP()` function writes a small char buffer and a timestamp — all single-word writes, safe without locks on ESP32. The actual I2C display calls happen in `updateDisplay()` on the main loop.
- **Overlay during edit mode**: If the user is editing when the network connects, we skip the overlay to avoid disrupting their input.
