## 1. OLED Indicator

- [x] 1.1 In `updateDisplay()`, after rendering the page content (and overlays), draw "SIM" in the top-right corner using `u8g2_font_helvR08_tr`, guarded by `#ifdef SIMULATE_OT`

## 2. API

- [x] 2.1 In `webserver.cpp`, add `doc["system"]["simulated"]` to the `/api/status` response, set to `true` under `#ifdef SIMULATE_OT` and `false` otherwise

## 3. Web UI

- [x] 3.1 Add `simulated` field to the status TypeScript interface in `api.ts`
- [x] 3.2 Show a persistent "Simulation Mode" banner at the top of the page when `simulated` is true (visible on all tabs)

## 4. Verification

- [ ] 4.1 Build sim firmware (manual) and verify "SIM" appears on OLED and banner appears on web UI
