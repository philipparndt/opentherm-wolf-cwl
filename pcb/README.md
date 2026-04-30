# Wolf CWL Controller — Shield PCB

Shield board that stacks on top of an [Olimex ESP32-POE](https://www.olimex.com/Products/IoT/ESP32/ESP32-POE/open-source-hardware) via the UEXT connector. Provides the OpenTherm interface, OLED display, and rotary encoder.

## UEXT Connector Pinout (ESP32-POE)

The [UEXT](https://www.olimex.com/Products/Modules/UEXT/) is a 10-pin 2x5 box header (2.54mm pitch) providing I2C, SPI, and UART:

| UEXT Pin | Signal | ESP32 GPIO | Used For |
|----------|--------|------------|----------|
| 1 | 3.3V | — | Power |
| 2 | GND | — | Ground |
| 3 | TXD | GPIO 1 | (not used) |
| 4 | RXD | GPIO 3 | (not used) |
| 5 | SCL | GPIO 16 | OLED Display |
| 6 | SDA | GPIO 13 | OLED Display |
| 7 | MISO | GPIO 15 | Encoder CLK |
| 8 | MOSI | GPIO 2 | Status LED |
| 9 | SCK | GPIO 14 | Encoder DT |
| 10 | SS | GPIO 5 | Encoder SW |

Additional signals from the ESP32-POE EXT1/EXT2 headers:

| Signal | ESP32 GPIO | Used For |
|--------|------------|----------|
| GPIO 36 | 36 | OpenTherm Input (RX) |
| GPIO 4 | 4 | OpenTherm Output (TX) |

## Bill of Materials

### Connectors

| Ref | Component | Mouser # | Manufacturer | Description | Qty |
|-----|-----------|----------|--------------|-------------|-----|
| J1 | HIF3FB-10DA-2.54DSA(69) | 798-HIF3FB10DA254D69 | Hirose | 2x5 female keyed IDC socket, 2.54mm, through-hole (mates with UEXT box header) | 1 |
| J2 | Screw Terminal | 651-1729018 | Phoenix Contact | 2-pos, 5.08mm, through-hole | 1 |
| J3 | — | — | — | 1x4 pads, 2.50mm pitch, for direct OLED soldering | 1 |
| J4 | B3B-XH-A(LF)(SN) | 306-B3B-XH-ALFSN | JST | 3-pin JST XH header, 2.50mm, GPIO4 + GPIO36 + GND | 1 |
| J5 | B4B-XH-A(LF)(SN) | 306-B4B-XH-ALFSN | JST | 4-pin JST XH header, 2.50mm, +3V3 + GND + TXD + RXD breakout | 1 |

### OpenTherm Interface

| Ref | Component | Mouser # | Manufacturer | Description | Qty |
|-----|-----------|----------|--------------|-------------|-----|
| U1, U2 | LTV-817S-B | 859-LTV-817S-B | Lite-On | Optocoupler, SOP-4 SMD (PC817 compatible) | 2 |
| Q1 | 2N7002 | 512-2N7002 | onsemi | N-MOSFET, SOT-23, 60V 200mA | 1 |
| D1 | BZX384-C4V7,115 | 771-BZX384-C4V7115 | Nexperia | 4.7V Zener, SOD-323, 300mW | 1 |
| D2-D5 | 1N4148WS | 512-1N4148WS | onsemi | Switching diode, SOD-323, 100V (bus protection, polarity-independent) | 4 |
| R1 | RC0603FR-07330RL | 603-RC0603FR-07330RL | Yageo | 330Ω, 0603, 1%, thick film | 1 |
| R2 | RC0603FR-07680RL | 603-RC0603FR-07680RL | Yageo | 680Ω, 0603, 1%, thick film | 1 |
| R3 | RC0603FR-074K7L | 603-RC0603FR-074K7L | Yageo | 4.7kΩ, 0603, 1%, thick film | 1 |
| R4, R6, R7 | RC0603FR-0710KL | 603-RC0603FR-0710KL | Yageo | 10kΩ, 0603, 1%, thick film | 3 |
| R5, R8 | RC0603FR-071KL | 603-RC0603FR-071KL | Yageo | 1kΩ, 0603, 1%, thick film | 2 |

### Display

| Ref | Component | Source | Description | Qty |
|-----|-----------|--------|-------------|-----|
| — | SH1106 1.3" OLED | AliExpress / Amazon | 128x64 I2C module, 4-pin | 1 |

### Encoder

| Ref | Component | Mouser # | Manufacturer | Description | Qty |
|-----|-----------|----------|--------------|-------------|-----|
| SW1 | EC12E24204A9 | 688-EC12E24204A9 | Alps Alpine | 24 detents, push switch, 20mm D-shaft | 1 |

### Status LED

| Ref | Component | Mouser # | Manufacturer | Description | Qty |
|-----|-----------|----------|--------------|-------------|-----|
| LED1 | SML-LXT0805GW-TR | 696-SML-LXT0805GW | Lumex | Green LED, 0805 SMD | 1 |

## OpenTherm Schematic

```
                   Shield                         OT Bus
                                                   ┌── OT+
  OT_TX_SIG ────── R1 330Ω ──┐                    │
                              LED+ U1 (LTV-817S)   │
                              LED-                 │
                       U1 collector ─┐             │
                       U1 emitter ── GND           │
                                     │             │
                       Q1 gate ──────┘             │
                       R3 4.7kΩ gate─GND           │
                       Q1 drain ───────────────────┤
                       Q1 source ── GND            │
                                                   │
  OT_RX_SIG ───┬── R4 10kΩ ── 3.3V               │
               │                                   │
               U2 collector                        │
               U2 emitter ── GND                   │
               │                                   │
               U2 LED+ ── R2 680Ω ────────────────┤
               U2 LED- ── R5 1kΩ ─────────────────┤
               D1 BZX384-C4V7 (zener) across LED   │
                                                   │
  Bus protection (polarity-independent):           │
               D2: OT+ ──(A)──(K)── +3V3          │
               D3: GND ──(A)──(K)── OT-           │
               D4: OT- ──(A)──(K)── +3V3          │
               D5: GND ──(A)──(K)── OT+           │
                                                   └── OT-

  Solder bridges (GPIO selection):
    SB3 (default closed): UEXT TXD (GPIO1) ── OT_TX_SIG
    SB4 (default closed): UEXT RXD (GPIO3) ── OT_RX_SIG
    SB1 (default open):   J4 GPIO4 ── OT_TX_SIG
    SB2 (default open):   J4 GPIO36 ── OT_RX_SIG
```

## Encoder Wiring

```
EC12E24204A9:

  Rotation side (3 pins):       Switch side (2 pins):
    Pin A (CLK) ─── GPIO 15      Pin 1 (SW) ─── GPIO 5
    Pin C (COM) ─── GND          Pin 2 (SW) ─── GND
    Pin B (DT)  ─── GPIO 14

  Optional: R6, R7 10kΩ pull-ups on CLK and DT to 3.3V
  (ESP32 internal pull-ups are enabled in firmware)
```

## KiCad Footprints

| Ref | KiCad Symbol | KiCad Footprint |
|-----|-------------|-----------------|
| J1 | Connector_Generic:Conn_02x05_Odd_Even | Connector_IDC:IDC-Header_2x05_P2.54mm_Vertical |
| J2 | Connector:Screw_Terminal_01x02 | TerminalBlock_Phoenix:TerminalBlock_Phoenix_MKDS-1,5-2-5.08_1x02_P5.08mm_Horizontal |
| J3 | Connector_Generic:Conn_01x04 | Connector_PinHeader_2.54mm:PinHeader_1x04_P2.54mm_Vertical |
| U1, U2 | Isolator:EL817 | Package_SO:SOP-4_3.8x4.1mm_P2.54mm |
| Q1 | Transistor_FET:2N7002 | Package_TO_SOT_SMD:SOT-23 |
| D1 | Diode:BZX384-C3V3 | Diode_SMD:D_SOD-323 |
| D2, D3 | Diode:1N4148W | Diode_SMD:D_SOD-323 |
| R1-R8 | Device:R | Resistor_SMD:R_0603_1608Metric |
| SW1 | Encoder:Encoder_Rotary_EC12E | Rotary_Encoder:RotaryEncoder_Alps_EC12E-Switch_Vertical_H20mm |
| LED1 | Device:LED | LED_SMD:LED_0805_2012Metric |

**Notes:**
- **J1 footprint**: The Hirose HIF3FB is a specific keyed connector — check Hirose's KiCad library or use the generic IDC footprint and verify pin spacing against the datasheet.
- **U1/U2 footprint**: The LTV-817S SOP-4 package has 2.54mm pin pitch (not standard SOP 1.27mm). Use `Package_SO:SOP-4_3.8x4.1mm_P2.54mm` or verify against the LTV-817S datasheet.
- **SW1 footprint**: The Alps EC12E has a specific 5-pin footprint (3 encoder + 2 switch). The KiCad library includes `RotaryEncoder_Alps_EC12E_Vertical_H20mm` for the 20mm shaft variant.
- **J3 footprint**: Match the pin spacing of your OLED module (typically 2.54mm, 4 pins in a row: GND, VCC, SCL, SDA).

## PCB Design Notes

- **Board size**: Match ESP32-POE footprint for clean stacking
- **UEXT**: 10-pin IDC socket on the bottom side, aligned with ESP32-POE UEXT header
- **EXT pins**: Route GPIO 36 and GPIO 4 via pin headers from ESP32-POE EXT1/EXT2
- **OLED**: 4-pin female header on the top edge, display faces up
- **Encoder**: Through-hole, positioned for front-panel access
- **Screw terminal**: Edge-mounted for easy OpenTherm bus wiring
- **2-layer PCB** is sufficient
- **All logic is 3.3V** (UEXT standard)
- **OpenTherm bus is polarity-independent**
