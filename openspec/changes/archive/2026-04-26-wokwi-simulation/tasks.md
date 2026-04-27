## 1. Simulation Mode

- [x] 1.1 Add `#ifdef SIMULATE_OT` blocks in `ot_master.cpp` — fake data generation replacing real OpenTherm communication
- [x] 1.2 Add `[env:esp32-sim]` to `platformio.ini` — copies esp32 env with `-DSIMULATE_OT=1`

## 2. Wokwi Configuration

- [x] 2.1 Create `wokwi.toml` — points to esp32-sim build output
- [x] 2.2 Create `diagram.json` — ESP32 + SSD1306 OLED (SDA=19, SCL=23) + KY-040 encoder (CLK=16, DT=17, SW=5)

## 3. Build Integration

- [x] 3.1 Add `make simulate` and `make build-sim` targets to Makefile
- [x] 3.2 Build verification — both `esp32` (production) and `esp32-sim` (simulation) build SUCCESS
