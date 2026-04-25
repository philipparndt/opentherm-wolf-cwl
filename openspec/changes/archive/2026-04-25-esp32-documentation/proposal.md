## Why

The ESP32 firmware project has no README documenting the hardware setup (wiring, components, pin assignments) or software setup (build, flash, configure). A new user looking at the `esp32/` directory needs to know what to buy, how to wire it, and how to get it running.

## What Changes

- Add `esp32/README.md` documenting:
  - Hardware components (ESP32 dev board, DIYLess OpenTherm Shield, SH1106 OLED, KY-040 encoder)
  - Wiring diagram (pin assignments for all components)
  - Software prerequisites (PlatformIO, Node.js)
  - Build and flash instructions
  - First-run configuration (AP mode, WiFi setup, MQTT)
  - Web UI and OTA updates
  - MQTT topic reference
  - OLED display pages and encoder controls

## Capabilities

### New Capabilities

(none — documentation only, no new firmware capabilities)

### Modified Capabilities

(none)

## Impact

- **New file**: `esp32/README.md`
- No code changes
