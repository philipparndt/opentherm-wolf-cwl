## Why

The main (home) screen currently only shows a single-letter suffix ("S" or "M") to distinguish schedule-driven ventilation from manual selection. This is easy to miss at a glance. The display should make it immediately obvious which mode is active by using a visually distinct layout or indicator for each state.

## What Changes

- Redesign the home screen layout so that schedule-active and manual-selection states are clearly distinguishable at a glance
- Use different visual elements (e.g. icons, borders, label positioning, or inverted sections) to convey the active mode without requiring the user to read small text
- Retain all existing information (ventilation level, bypass status) while making mode status more prominent

## Capabilities

### New Capabilities
- `schedule-display-mode`: Visual differentiation of the home screen when schedule is active vs manual override

### Modified Capabilities

## Impact

- `esp32-rust/src/display.rs` — `draw_home()` function and related rendering logic
- No protocol or MQTT changes
- No hardware changes — same 128x64 OLED display
