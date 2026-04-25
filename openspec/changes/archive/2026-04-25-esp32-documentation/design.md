## Context

The `esp32/` directory contains a complete PlatformIO firmware project with Preact web UI. All hardware details (pin assignments, component choices) are documented in the code via `platformio.ini` build flags and `include/config.h`. This change extracts that knowledge into a user-facing README.

## Goals / Non-Goals

**Goals:**
- Single `esp32/README.md` covering hardware and software setup
- Accurate pin assignments matching `platformio.ini` build flags
- ASCII wiring table (no images required)
- Copy-paste build commands
- MQTT topic reference table for Home Assistant integration

**Non-Goals:**
- Separate hardware assembly guide with photos
- PCB design files
- Home Assistant YAML configuration examples (that's a separate project)
