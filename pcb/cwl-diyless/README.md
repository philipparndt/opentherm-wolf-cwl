# Wolf CWL Shield — `cwl-diyless` variant

DIYless-based variant of the shield: no on-board OpenTherm circuit, a soldered-on DIYless D1 mini OT shield instead, and both 0.96" (J3) and 1.3" (J3B) OLED footprints on the same board (populate one).

## What's on this board

| Ref       | Component                                  | Notes                                                                 |
|-----------|--------------------------------------------|-----------------------------------------------------------------------|
| J1        | UEXT 2×5 IDC socket (Olimex stack)         | Same as the original variant                                          |
| J3        | OLED 0.96" SSD1306 footprint               | Pinout pad 1→4: GND, VCC, SCL, SDA                                    |
| J3B       | OLED 1.3" SH1106 footprint                 | Pinout pad 1→4: VDD, GND, SCK, SDA — populate **either J3 or J3B**    |
| J4        | EXT GPIO breakout (GPIO 4 / GPIO 36 / GND) | JST XH 3-pin                                                          |
| J5        | Aux breakout (3V3 / GND / TXD / RXD)       | JST XH 4-pin — useful for external logic-probe access                 |
| J6        | DIYless D1 mini left header (1×8)          | Pins NC except P8 = 3V3                                               |
| J7        | DIYless D1 mini right header (1×8)         | P3 = D1 (OT TX), P4 = D2 (OT RX), P7 = GND, P8 = 5V (NC by default)   |
| J8        | UEXT debug 1×10 pin header                 | All 10 UEXT signals broken out for scope/LA hook-up                   |
| SW1       | Alps EC12E rotary encoder + switch         | Same as the original variant                                          |
| LED1 + R8 | Status LED + 1 kΩ series resistor          | Same as the original variant                                          |
| MH1–MH4   | M3 mounting holes (GND-stitched)           |                                                                       |

**Omitted vs. `cwl/`:** R1–R5, R8 (kept), U1, U2, Q1, D1–D5 (on-board OT optocoupler interface), J2 (screw terminal), SB1–SB4 (OT routing solder bridges). The DIYless shield carries all of those functions and its own bus terminal.

## D1 mini layout reminder for the PCB editor

J6 (left) and J7 (right) are placed at the standard D1 mini header spacing — **22.86 mm (0.9") between row centerlines**, both rows running parallel along the long axis of the shield. The shield's 16 male pins drop straight into the PTH pads.

The five pins that carry actual signal are:

```
   J6 (left)             J7 (right)
   ┌───┐                ┌───┐
   │ 1 │ RST            │ 1 │ TX
   │ 2 │ A0             │ 2 │ RX
   │ 3 │ D0             │ 3 │ D1   →  ot_tx_sig (UEXT TXD, GPIO 4)
   │ 4 │ D5             │ 4 │ D2   →  ot_rx_sig (UEXT RXD, GPIO 36)
   │ 5 │ D6             │ 5 │ D3
   │ 6 │ D7             │ 6 │ D4
   │ 7 │ D8             │ 7 │ GND
   │ 8 │ 3V3            │ 8 │ 5V   (leave NC, or jumper-wire externally if
   └───┘                └───┘       your DIYless revision needs 5V on Vin)
```

## Workflow

```bash
# Generate the netlist
make generate VARIANT=cwl-diyless

# In KiCad: create a new project at pcb/cwl-diyless/cwl.kicad_pro,
#          open the PCB editor, File → Import Netlist → wolf-cwl-shield-cwl-diyless.net,
#          place components (use the D1 mini spacing above for J6/J7),
#          route, then back to the top level for Gerbers:

make gerber VARIANT=cwl-diyless
make plot-pdf VARIANT=cwl-diyless
```

## Firmware

Use the existing `../../esp32/` build (Olimex target). Flash with `make prod-wifi` (or `make dev-wifi` for simulated OT). No firmware changes needed — pin assignments via `ot-ext` already match GPIO 4 / GPIO 36.
