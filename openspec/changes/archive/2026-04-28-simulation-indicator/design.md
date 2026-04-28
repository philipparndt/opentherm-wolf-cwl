## Context

The `SIMULATE_OT` build flag enables a simulated OpenTherm master that generates fake temperature and ventilation data. This flag is set in the `esp32-sim` and `esp32-poe-sim` PlatformIO environments. Currently nothing in the UI (OLED or web) indicates simulation mode.

## Goals / Non-Goals

**Goals:**
- Make it immediately obvious on the OLED that the device is running in simulation mode.
- Make it immediately obvious on the web UI that the data is simulated.
- Zero impact on non-simulation builds.

**Non-Goals:**
- Runtime toggling of simulation mode.
- Detailed simulation controls or configuration.

## Decisions

**Draw "SIM" in the top-right corner of the OLED on every page.**

A small label using the `u8g2_font_helvR08_tr` font, drawn after the page content so it overlays consistently. Wrapped in `#ifdef SIMULATE_OT` so it compiles out in production builds. Also shown during overlays and the CWL disconnected screen.

**Add `"simulated": true/false` to the existing `/api/status` response.**

Use a compile-time check: `#ifdef SIMULATE_OT` sets it to `true`, otherwise `false`. No new endpoint needed — extends the existing status object under the `system` key.

**Show a banner/badge on the web UI when `simulated` is true.**

A persistent colored banner (e.g., orange/yellow) at the top of the page indicating "Simulation Mode". Visible on all tabs so it can't be missed.

## Risks / Trade-offs

- **Screen real estate on OLED**: "SIM" in the top-right is small (3 chars in 8px font ≈ 16px wide) and won't overlap with page headers which are left-aligned.
