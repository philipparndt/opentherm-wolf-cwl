# Wolf CWL Controller — Debug Notes

Capture of a multi-day debug session on the v1.x PCB. Lots of real bugs were fixed along the way; one stubborn one (CWL doesn't respond) is still open at the end.

## TL;DR — what's fixed, what's open

| Status | Issue | Where | Fix |
|--------|-------|-------|-----|
| ✅ | D2/D4 shunt-clamp the OT bus to Vcc+0.7V | `pcb/generate_schematic.py:244-256` | Remove D2, D3, D4, D5 from board |
| ✅ | D3 was actually needed as GND→OT- return path | same | Re-install D3 only (anode=GND, cathode=OT-) |
| ✅ | R3 wired as pulldown — Q1 gate never reaches Vth | TX path on PCB | Lift R3 GND side, fly-wire to Vcc |
| ✅ | `ot-uext` feature targets wrong GPIO pins | `esp32/Cargo.toml` features | Switch default to `ot-ext` (GPIO4/GPIO36) |
| ✅ | Project README claimed UEXT TXD/RXD = GPIO1/GPIO3 | `pcb/README.md` | Corrected to GPIO4/GPIO36 (actual Olimex pinout) |
| ✅ | Missing `partitions.csv` (gitignored by accident) | `.gitignore` | Negated `*.csv` for `esp32/partitions.csv` |
| ✅ | `build.rs` uses host `ar/ranlib` for Xtensa objects | `esp32/build.rs` | Added `.archiver(xtensa-esp32-elf-ar)` |
| ✅ | First-time setup pain (ldproxy, espflash, python venv) | `esp32/Makefile` | `make setup`, `make fix-python-env` |
| ✅ | Decoder couldn't handle Saleae digital CSV exports | `decode/reader/csv.go` | Detect 0/1 transition format, expand+invert+scale |
| ⚠️ | **CWL still doesn't respond** | open | see [Outstanding issue](#outstanding-issue) |

## Olimex ESP32-POE pinout (correct version)

The original schematic and README assumed Olimex's UEXT TXD/RXD were the ESP32's UART0 pins (GPIO1/GPIO3). **They aren't.** Olimex routes GPIO1/GPIO3 to the on-board USB-serial converter for debug, and brings out *different* GPIOs on UEXT:

| UEXT Pin | Signal | ESP32 GPIO |
|----------|--------|------------|
| 3 (TXD)  | UART   | **GPIO 4** |
| 4 (RXD)  | UART   | **GPIO 36** |

So `ot-uext` (drives GPIO1) and `ot-ext` (drives GPIO4) end up routing to the same physical net via different solder bridges; the firmware just needs to drive the pin that's actually connected. Default feature flag is now `ot-ext`.

## Hardware bugs (PCB v1.x)

### 1. D2-D5 "polarity protection" is mis-wired

`generate_schematic.py:244-256` calls D2-D5 a "full-bridge" but it's not a bridge — it's four shunt diodes from bus pins to the rails:

```
D2: OT+  → +3V3   (forward conducts when OT+ > Vcc+0.7V ≈ 4V)
D3: GND  → OT-    (forward conducts when GND  > OT-+0.7V) ← actually useful
D4: OT-  → +3V3   (forward conducts when OT-  > Vcc+0.7V ≈ 4V)
D5: GND  → OT+    (forward conducts when GND  > OT++0.7V) ← reverse-polarity only
```

**Effect:** With CWL connected at correct polarity, D2 clamps OT+ to ~4V. The bus collapses (~16V → ~7V) and OT signaling cannot work. Confirmed by multimeter: 16.5V open → 6.9V with PCB.

**Fix:** Remove D2, D3, D4, D5. Polarity is now fixed (the bridge was nominally what gave polarity tolerance), so the user wires must be the right way around.

### 2. ...but D3 turned out to be load-bearing

After removing all four, bus rose to ~10V (not the expected ~16V), and SB4 sat at 0V — RX opto LED conducted continuously, blind to bus modulation. The reason: **PCB GND had no return path to OT-**, so Q1 turning ON couldn't actually sink current from the bus.

D3 (GND→OT- via diode) provides exactly that return path. We removed it as part of the "polarity bridge" cleanup, not realizing it was the only DC return.

**Fix:** Re-install **D3 only** (1N4148WS). Anode pad → GND, cathode (band) → OT-. Verify with diode-test mode probing from any GND test point to the OT- screw terminal (forward ~0.6V, reverse OL).

Do *not* re-install D2, D4, D5.

Trade-off: with D3 as diode (vs. a wire jumper) the master signal path has a 0.7V drop, but reverse polarity is still safe (D3 reverse-blocks and Q1 body diode clamps PCB GND near the wrong-side terminal — bus current can't flow, but nothing fries).

### 3. R3 was pulldown — should be pullup

`generate_schematic.py:209-210`:

```
r3 connects tx_gate to GND
```

`tx_gate` is the output of the TX opto's collector (emitter is GND, so when LED conducts, transistor pulls `tx_gate` to GND). With R3 also pulling to GND, the Q1 gate sits at 0V in both states → Q1 never conducts → master can never sink current → CWL never sees TX.

**Why it's wrong:** the opto-coupler is acting as a logic-level inverter (output stage open-collector to GND), so the gate needs a pullup to Vcc for the off-state.

**Fix:** Lift R3's GND side, fly-wire to Vcc (e.g., the Vcc-side pad of R4 in the RX path is only a few mm away). Verify: tx_gate idle should now be ~3.2 V (was 0 V before).

After this fix, the TX path was proven end-to-end:
- TX_gate idles HIGH (3.24 V), drops cleanly during Manchester bursts.
- Saleae at SB3 / J5 P3 (UEXT_TXD = GPIO4) shows clean 34-bit bursts ~once per second.
- `decode` tool confirms valid Manchester encoding at 999 baud (spec 1000 baud), correct OT framing, correct parity on 4/5 packets, data values match what `poll_cycle` builds.

## Firmware fixes

### `Cargo.toml`

- Default features changed: `default = ["wifi", "ot-ext", "display-rotate"]` (was `ot-uext`).
- Marked `ot-uext` as deprecated/wrong — it targets GPIO1/3 which aren't on the Olimex UEXT.

### `build.rs`

- Forwards `WIFI_SSID` / `WIFI_PASSWORD` from env to compile-time `env!()` so `make dev-wifi` / `prod-wifi` can bake fallback credentials.
- Added `.archiver(xtensa-esp32-elf-ar)` — without this, `cc` falls back to host (Mach-O) `ar/ranlib` on the Xtensa ELF objects, producing an empty archive → undefined `ot_init` / `ot_send_request` at final link.

### `main.rs`

- Compile-time WiFi credentials fallback when NVS has none (lets a freshly-flashed board join the network without going through the web setup).
- OT thread stack bumped from 8 KB to 16 KB (defensive; wasn't the actual bug).

### `Makefile`

New targets:
- `make setup` — `cargo install ldproxy espflash`, plus `fix-python-env`.
- `make dev-wifi` / `make prod-wifi` — load `WIFI_SSID`/`WIFI_PASSWORD` from `esp32/.env`.
- `make fix-python-env` — auto-applied before every `build`. Symlinks `ruamel_yaml-*.dist-info` → `ruamel.yaml-*.dist-info` to work around Python 3.9's `importlib.metadata` not normalizing dotted names → the ESP-IDF dep check otherwise exits 255 and CMake aborts.

### `partitions.csv`

Was referenced by Makefile but absent from git. Now committed; `.gitignore`'s `*.csv` rule has an explicit `!esp32/partitions.csv` negation.

### `decode/reader/csv.go`

Saleae digital exports are transition-only (Time, 0|1 at edges, not regular samples) — the original analog-CSV reader couldn't consume them. New code detects all-binary values, expands to 100 kHz regular samples, inverts if dominant state is HIGH, and rescales to 0/20 V so the existing analog Manchester decoder pipeline works unchanged.

## Outstanding issue

**Symptom:** OLED shows "CWL Disconnected" indefinitely. Bus at ~10V when PCB connected (should be ~16V).

### What we know works

1. **Master TX is spec-conformant.** Saleae capture decoded via `decode digital.csv --verbose`:
   - Manchester 999 baud (spec 1000, 0.1% off)
   - 4/5 packets: correct framing, correct parity, data values match `poll_cycle`'s `build_request` calls
   - ID sequence matches firmware poll order (82 → 126 → 127 → 89 → 72 → ...)
2. **R3 fix verified by measurement** (tx_gate = 3.24V idle, drops during bursts).
3. **D3 forward voltage = 0.586V** in diode-test mode — diode itself is healthy.

### What's still wrong

After D3 re-install, multimeter between PCB GND and OT- reads **~9.5V** with a small swing toward 9.98V during each 1-second burst. Expected: ~0V (D3 forward-biased clamps GND to OT- + 0.7V).

This means D3 isn't actually conducting in-circuit. Most likely causes:
- D3 installed in the wrong orientation despite the schematic intent (silkscreen ambiguity?)
- Cold solder joint on one of D3's pads
- Or D3's pad isn't routed to the net we think it is on the actual PCB

**Diagnostic step (not yet run):** PCB stripped of bus and USB, multimeter in diode-test mode:
- Red on PCB GND test point, black on OT- screw → expect ~0.6V
- Swap → expect OL
- Both OL → D3 backwards or open
- Both 0.6V → unlikely short

### Hypothesis stack (in case D3 path is fine)

Even with D3 conducting, there may be a second-order problem:

**Master idle current too low.** Spec OT master idles in "high current" state (17–23 mA). Our circuit at idle has only the RX path conducting (~8 mA), which is OT "low current" state. The Wolf CWL might interpret this as "master is stuck sending 0" and refuse to respond.

The cleanest fix would be a firmware patch in the C++ OT lib to invert `setIdleState` (drive GPIO LOW so Q1 stays ON between frames, sinking ~25 mA continuously and only briefly dropping during TX active-half-bits). The existing `OpenTherm.cpp` does:

```cpp
void OpenTherm::setIdleState()  { digitalWrite(outPin, HIGH); }  // ← wrong for our circuit
void OpenTherm::setActiveState(){ digitalWrite(outPin, LOW);  }
```

…which assumes a *non-inverting* output stage. Our PCB inverts via the opto buffer, so the convention is flipped. Either:
- Swap the two function bodies in our copy of OpenTherm.cpp, **or**
- Re-design the TX stage to be non-inverting (one more transistor).

### Open questions to investigate

1. Why does D3 not conduct? (Re-check orientation, joints, trace.)
2. Once D3 conducts: does the CWL respond? (Watch SB4 for activity 20–100ms after each TX burst.)
3. If CWL still silent: is it because master idle is "low current"? (Patch the lib's idle convention and re-test.)
4. Does the CWL need a particular Init frame before it'll talk? (The poll cycle starts with `MasterConfig` (ID 2) — that's the standard init. But worth verifying with a packet capture from a known-working master/remote.)

## Measurement log (chronological)

| Step | Configuration | Measurement | Value | Comment |
|------|---------------|-------------|-------|---------|
| Baseline | No PCB, multimeter on CWL terminals | OT+ – OT- DC | 16.5 V | Healthy bus |
| Initial connect | PCB with D2-D5 populated | Same terminals | 6.9 V | D2/D4 clamping |
| Diodes removed | D2-D5 lifted | Same terminals | 10.3 V | Better, but still loaded |
| Single-ended | D2-D5 lifted, probes vs PCB GND | OT+ vs GND | 9.5 V | |
| | | OT- vs GND | ~0 V + noise | |
| TX path static | After R3→Vcc fly-wire | Q1 gate (tx_gate) vs GND | 3.24 V | Inverter logic now correct |
| TX path dynamic | Same; Saleae at GPIO4 | digital, 1 MS/s | 34-bit bursts @ 1 Hz | TX confirmed end-to-end |
| TX decode | `digital.csv` through `decode` | Manchester baudrate | 999 baud | Within 0.1% of spec |
| | | Parity on 4/5 packets | OK | First packet probably trigger artifact |
| | | Data values vs firmware | match | poll_cycle state machine confirmed |
| RX path | D3 re-installed, with CWL | Saleae at SB4 (GPIO36) | 0 V steady | Opto saturated by bus current |
| Return path | D3 in place | Multimeter GND vs OT- | 9.5 V | **D3 not actually conducting** |

## Saleae probe-point cheat sheet

For future debugging — useful PCB nets and where to probe them:

| Net | Probe at | Voltage range | Idle | During TX |
|-----|----------|---------------|------|-----------|
| `ot_tx_sig` (GPIO4) | SB3 or J5 P3 | 0 – 3.3 V CMOS | HIGH | Manchester burst |
| `tx_gate` (Q1 G) | Q1 gate pad / R3 free end | 0 – 3.3 V | HIGH (3.24V, R3→Vcc) | inverted of GPIO4 |
| `ot_rx_sig` (GPIO36) | SB4 or J5 P4 | 0 – 3.3 V CMOS | HIGH at 3.2 V (idle, R4 pullup) | LOW during slave high voltage |
| OT+ vs OT- | CWL screw terminals | 0 – 24 V analog | ~16 V open, ~10 V loaded | dips during master sink |

Always:
- Saleae GND on **PCB GND** (J5 P2 or any GND test point), never on OT-.
- Digital sample rate 1 MS/s is plenty (Manchester half-bit is 500µs → 500 samples).
- Trigger Ch0 falling edge on GPIO4 to catch a burst within ~1 second.
- Use Saleae's **Export Raw Data → CSV** for digital channels — the `decode` tool now handles them directly.

## Lessons for the next PCB revision

1. **Polarity protection via a real bridge rectifier (Graetz) in *series* with the bus pair**, not four shunt diodes. Or accept fixed polarity and use a single Schottky in the return path (= D3 done correctly).
2. **TX stage should be non-inverting** so the OT library's convention matches (avoid the R3-to-Vcc fly-wire).
3. **RX comparator with a threshold around 10 V**, not a fixed opto-LED current path. The current circuit can't differentiate slave-high (16 V) from slave-low (5 V) because both produce enough opto LED current to saturate.
4. **Document the actual Olimex pin mapping** in the schematic comments. Don't trust assumptions about what "UEXT TXD" maps to on a specific board.
5. **Drop `ot-uext` feature** from firmware — it never made sense given the actual pin mapping.

## Files touched in this session

```
esp32/Cargo.toml          # default features, ot-uext deprecated
esp32/Makefile            # setup, dev-wifi, prod-wifi, fix-python-env
esp32/build.rs            # archiver fix, WiFi env forwarding
esp32/main.rs             # OT thread stack 8192→16384, WiFi fallback
esp32/sdkconfig.defaults  # (CONFIG_ESP_CONSOLE_NONE added then removed)
esp32/partitions.csv      # newly committed (was gitignored)
esp32/.env.example        # WIFI_SSID, WIFI_PASSWORD, PORT
esp32/README.md           # new — full firmware docs
.gitignore                # !esp32/partitions.csv negation
pcb/README.md             # UEXT pinout corrected
decode/reader/csv.go      # digital CSV support
```
