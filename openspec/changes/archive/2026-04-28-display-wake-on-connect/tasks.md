## 1. Display IP Overlay

- [x] 1.1 Add `displayShowIP(const char* ip)` declaration to `display.h`
- [x] 1.2 Add static state variables in `display.cpp`: IP string buffer (16 chars), overlay start timestamp, overlay active flag
- [x] 1.3 Implement `displayShowIP()` in `display.cpp`: call `displayWake()`, copy IP into buffer, record `millis()` timestamp, set overlay active
- [x] 1.4 Add overlay rendering in `updateDisplay()`: if overlay active and within 10 seconds, draw centered IP screen (header "Connected", large IP text) instead of normal page; clear overlay after 10 seconds
- [x] 1.5 Clear overlay on encoder input: in `displayWake()`, clear the overlay flag so user interaction dismisses it

## 2. Network Integration

- [x] 2.1 Add `#include "display.h"` to `network.cpp`
- [x] 2.2 Call `displayShowIP()` in `onEthEvent()` on `ARDUINO_EVENT_ETH_GOT_IP` (ethernet build)
- [x] 2.3 Call `displayShowIP()` in `networkLoop()` when WiFi transitions from disconnected to connected (WiFi build)

## 3. Verification

- [ ] 3.1 Build firmware and verify no compilation errors
