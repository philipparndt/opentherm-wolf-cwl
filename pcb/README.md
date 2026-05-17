# Wolf CWL Controller — Shield PCB

Shield board that stacks on top of an [Olimex ESP32-POE](https://www.olimex.com/Products/IoT/ESP32/ESP32-POE/open-source-hardware) via the UEXT connector. Provides the OpenTherm interface, OLED display, and rotary encoder.

## UEXT Connector Pinout (ESP32-POE)

The [UEXT](https://www.olimex.com/Products/Modules/UEXT/) is a 10-pin 2x5 box header (2.54 mm pitch) providing I²C, SPI, and UART:

| UEXT Pin | Signal | ESP32 GPIO | Used For |
|----------|--------|------------|----------|
| 1  | 3.3V | —       | Power |
| 2  | GND  | —       | Ground |
| 3  | TXD  | GPIO 4  | OpenTherm TX (via SB3) |
| 4  | RXD  | GPIO 36 | OpenTherm RX (via SB4) |
| 5  | SCL  | GPIO 16 | OLED Display |
| 6  | SDA  | GPIO 13 | OLED Display |
| 7  | MISO | GPIO 15 | Encoder CLK |
| 8  | MOSI | GPIO 2  | Status LED |
| 9  | SCK  | GPIO 14 | Encoder DT |
| 10 | SS   | GPIO 5  | Encoder SW |

## Variants

| Variant | Project | Notes |
|---------|---------|-------|
| `cwl` (default) | `cwl/cwl.kicad_pro` | Main board. Melnyk-topology on-board OpenTherm front-end. Combined OLED footprint accepts either a 0.96″ or 1.3″ SH1106 module. |
| `cwl-diyless` | `cwl-diyless/cwl.kicad_pro` | Off-board OpenTherm interface via a DIYless D1-mini shield. Separate J3 / J3B footprints for the two OLED sizes. |

## Bill of Materials (`cwl`)

| Ref | Part | Value / Mfr P/N | Footprint | Symbol | Mouser # | Qty |
|-----|------|-----------------|-----------|--------|----------|-----|
| J1 | UEXT 2×5 IDC socket | HIF3FB-10DA-2.54DSA(69) (Hirose) | `IDC:IDC-Stecker_2x05_P2.54mm_Vertical` | `Connector_Generic:Conn_02x05_Odd_Even` | 798-HIF3FB10DA254D69 | 1 |
| J2 | Screw terminal, 2-pos 5.08 mm | MKDS 1,5/2-5,08 (Phoenix Contact) | `TerminalBlock_Phoenix:TerminalBlock_Phoenix_MKDS-1,5-2-5.08_1x02_P5.08mm_Horizontal` | `Connector:Conn_01x02_Pin` | 651-1729018 | 1 |
| J3 | OLED header (0.96″ + 1.3″ combined) | 4-pin header (use module's own header) | `SSD1306:128x64OLED-MountingHoles-Combined` | `Connector:Conn_01x04_Pin` | — | 1 |
| J4 | EXT-GPIO breakout, 3-pin JST XH | B3B-XH-A(LF)(SN) (JST) | `Connector_JST:JST_XH_B3B-XH-A_1x03_P2.50mm_Vertical` | `Connector:Conn_01x03_Pin` | 306-B3B-XH-ALFSN | 1 |
| J5 | AUX breakout, 4-pin JST XH | B4B-XH-A(LF)(SN) (JST) | `Connector_JST:JST_XH_B4B-XH-A_1x04_P2.50mm_Vertical` | `Connector:Conn_01x04_Pin` | 306-B4B-XH-ALFSN | 1 |
| U1, U2 | Optocoupler, SOIC-4 | LTV-817S-B (Lite-On) | `Package_SO:SOP-4_7.5x4.1mm_P2.54mm` | `Isolator:PC817` | 859-LTV-817S-B | 2 |
| Q1 | PNP BJT, SOT-23 | BC858A or BC858B (Nexperia) | `Package_TO_SOT_SMD:SOT-23` | `Transistor_BJT:BC858` | 771-BC858A,215 | 1 |
| D1–D4 | Switching diode, SOD-323F | 1N4148WS (Diodes Inc) | `Diode_SMD:D_SOD-323F` | `Device:D` | 621-1N4148WS-7-F | 4 |
| D5 | 4.7 V zener, SOD-123, 500 mW | BZT52C4V7 (Nexperia) | `Diode_SMD:D_SOD-123` | `Device:D_Zener` | 771-BZT52C4V7,115 | 1 |
| D6 | 15 V zener, SOD-123, 500 mW | BZT52C15 (Nexperia) | `Diode_SMD:D_SOD-123` | `Device:D_Zener` | 771-BZT52C15,115 | 1 |
| D7 | 4.3 V zener, SOD-123, 500 mW | BZT52C4V3 (Nexperia) | `Diode_SMD:D_SOD-123` | `Device:D_Zener` | 771-BZT52C4V3,115 | 1 |
| R1, R4 | 330 Ω, 0603, 1 % | RC0603FR-07330RL (Yageo) | `Resistor_SMD:R_0603_1608Metric` | `Device:R` | 603-RC0603FR-07330RL | 2 |
| R2 | 220 Ω, 0603, 1 % | RC0603FR-07220RL (Yageo) | `Resistor_SMD:R_0603_1608Metric` | `Device:R` | 603-RC0603FR-07220RL | 1 |
| R3 | 100 Ω, 0603, 1 % | RC0603FR-07100RL (Yageo) | `Resistor_SMD:R_0603_1608Metric` | `Device:R` | 603-RC0603FR-07100RL | 1 |
| R5 | 1.5 kΩ, 0603, 1 % | RC0603FR-071K5L (Yageo) | `Resistor_SMD:R_0603_1608Metric` | `Device:R` | 603-RC0603FR-071K5L | 1 |
| R6, R7 | 10 kΩ, 0603, 1 % | RC0603FR-0710KL (Yageo) | `Resistor_SMD:R_0603_1608Metric` | `Device:R` | 603-RC0603FR-0710KL | 2 |
| R8 | 1 kΩ, 0603, 1 % | RC0603FR-071KL (Yageo) | `Resistor_SMD:R_0603_1608Metric` | `Device:R` | 603-RC0603FR-071KL | 1 |
| SW1 | Rotary encoder w/ switch | EC12E2424407 (Alps Alpine) | `Rotary_Encoder:RotaryEncoder_Alps_EC12E-Switch_Vertical_H20mm` | `Device:RotaryEncoder_Switch` | — | 1 |
| LED1 | Green LED, 0805 | SML-LXT0805GW-TR (Lumex) | `LED_SMD:LED_0805_2012Metric` | `Device:LED` | 696-SML-LXT0805GW | 1 |
| MH1–MH4 | M3 mounting hole, GND-tied | — | `MountingHole:MountingHole_3.2mm_M3_Pad_Via` | `Mechanical:MountingHole_Pad` | — | 4 |
| SB1–SB4 | 0603 solder bridges (etched into board, no part) | — | `Jumper:SolderJumper-2_P1.3mm_*` | `Jumper:SolderJumper_2_Open` / `_Bridged` | — | — |
| — | 0.96″ SH1106 OLED module, I²C, 4-pin | — | — | — | (Amazon / AliExpress) | 1 (pick one) |
| — | 1.3″ SH1106 OLED module, I²C, 4-pin | — | — | — | (Amazon / AliExpress) | 1 (pick one) |

## Build targets

```
make generate-3d      # regenerate the local STEP models in 3dmodels/
make export-stl-full  # full PCB assembly STL (VARIANT=cwl by default)
make export-stl-print # version with floating IDC socket for 3D printing
make export-stl-board # bare PCB only
make export-stl-parts # components only (no board)
make gerber           # Gerbers + drills (zipped)
make plot-pdf         # front + back assembly PDFs
make ibom             # Interactive HTML BOM (needs InteractiveHtmlBom plugin)
make clean            # remove generated artifacts
```

Set `VARIANT=cwl-diyless` to target the diyless project instead.
