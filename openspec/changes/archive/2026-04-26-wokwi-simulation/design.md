## Context

Wokwi simulates ESP32 hardware in a browser. It integrates with PlatformIO via a CLI tool (`wokwi-cli`) or the VS Code extension. The simulator supports: ESP32 dev boards, SH1106/SSD1306 I2C OLEDs, rotary encoders, LEDs, and WiFi (via a virtual gateway). It reads a `diagram.json` for the circuit layout and `wokwi.toml` for configuration.

The firmware's OpenTherm communication uses interrupts and the DIYLess shield — these can't be simulated. Instead, when `SIMULATE_OT` is defined, `ot_master.cpp` skips the real OpenTherm library and populates `cwlData` with fake values that change over time to look realistic.

## Goals / Non-Goals

**Goals:**
- `make simulate` launches the Wokwi simulation with OLED + encoder visible
- Fake OT data: temperatures vary sinusoidally (15-25°C), ventilation % responds to level changes, status flags toggle, TSP registers populated with realistic values
- Full firmware otherwise identical: WiFi, MQTT, web UI, schedules, watchdog all work
- Display pages render correctly with simulated data
- Encoder clicks and rotations work in the Wokwi UI

**Non-Goals:**
- Simulating the actual OpenTherm electrical protocol
- Simulating the DIYLess shield hardware
- Running without Wokwi CLI installed (it's an opt-in dev tool)

## Decisions

### 1. Simulation mode via `#ifdef SIMULATE_OT`

In `ot_master.cpp`, wrap `setupOpenTherm()` and `openThermLoop()` with `#ifdef SIMULATE_OT` / `#else` blocks. In simulation mode:
- `setupOpenTherm()` logs "SIMULATION MODE" and sets `cwlData.connected = true`
- `openThermLoop()` updates `cwlData` with fake values every second
- `probeAdditionalIds()` marks all additional IDs as supported
- No OpenTherm library calls, no interrupt handler

### 2. Fake data generation

```
supplyInletTemp:   18.0 + 5.0 * sin(millis/60000)     // Oscillates 13-23°C
exhaustInletTemp:  21.0 + 2.0 * sin(millis/45000)     // Oscillates 19-23°C
relativeVentilation: maps from ventilationLevel (0→0, 1→51, 2→67, 3→100)
fault: false (toggles briefly every 5 minutes to test display)
filterDirty: false
ventilationActive: follows requestedBypassOpen
TSP registers: static realistic values from README
```

### 3. `[env:esp32-sim]` in `platformio.ini`

Copies `[env:esp32]` but adds `-DSIMULATE_OT=1` to build flags. Same board, same libs, same partitions.

### 4. Wokwi files

`wokwi.toml`:
```toml
[wokwi]
version = 1
firmware = ".pio/build/esp32-sim/firmware.bin"
elf = ".pio/build/esp32-sim/firmware.elf"
```

`diagram.json`: ESP32 dev board with SH1106 OLED on I2C (SDA=19, SCL=23) and KY-040 encoder (CLK=16, DT=17, SW=5).

### 5. Makefile target

```makefile
simulate: build-sim
    wokwi-cli .

build-sim:
    $(PIO) run -e esp32-sim
```

Requires `wokwi-cli` installed (`npm install -g @anthropic-ai/wokwi-cli` or via pip). The user can also use the VS Code Wokwi extension instead.
