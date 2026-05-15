"""
Wolf CWL OpenTherm Shield — KiCad netlist generator using SKiDL.

Run with: python generate_schematic.py [--variant=NAME]
Or:       VARIANT=cwl-diyless python generate_schematic.py

Variants:
  cwl          (default) — 0.96" OLED, on-board OT optocoupler/MOSFET circuit
  cwl-1.3      — same circuit, 1.3" OLED footprint (set in PCB editor)
  cwl-diyless  — no on-board OT (use a DIYless D1 mini OT shield instead),
                 both 0.96" and 1.3" OLED footprints (populate one)

Output: wolf-cwl-shield.net (KiCad netlist)
"""

import os
import sys
import warnings
warnings.filterwarnings("ignore")

from skidl import *

# Variant selection — env var or `--variant=` CLI flag
VARIANT = os.environ.get("VARIANT", "cwl")
for arg in sys.argv[1:]:
    if arg.startswith("--variant="):
        VARIANT = arg.split("=", 1)[1]
if VARIANT not in ("cwl", "cwl-1.3", "cwl-diyless"):
    raise SystemExit(f"Unknown VARIANT: {VARIANT!r} (expected cwl, cwl-1.3, or cwl-diyless)")

# Suppress SKiDL warnings about missing KiCad libs
import logging
logging.getLogger("skidl").setLevel(logging.ERROR)

# =============================================================================
# Part factories (inline definitions, no external libraries needed)
# =============================================================================

def make_connector(ref, value, pin_count, footprint):
    pins = [Pin(num=str(i), name=f"P{i}", func=Pin.types.PASSIVE) for i in range(1, pin_count + 1)]
    return Part(tool=SKIDL, name=value, ref=ref, value=value, footprint=footprint, pins=pins)

def R(ref, value):
    return Part(tool=SKIDL, name="R", ref=ref, value=value,
                footprint="Resistor_SMD:R_0603_1608Metric",
                pins=[Pin(num="1", name="1", func=Pin.types.PASSIVE),
                      Pin(num="2", name="2", func=Pin.types.PASSIVE)])

def OPTO(ref, value):
    return Part(tool=SKIDL, name="LTV-817S", ref=ref, value=value,
                footprint="Package_SO:SOP-4_7.5x4.1mm_P2.54mm",
                pins=[Pin(num="1", name="A", func=Pin.types.PASSIVE),
                      Pin(num="2", name="K", func=Pin.types.PASSIVE),
                      Pin(num="3", name="E", func=Pin.types.PASSIVE),
                      Pin(num="4", name="C", func=Pin.types.PASSIVE)])

def NMOS(ref, value):
    return Part(tool=SKIDL, name="2N7002", ref=ref, value=value,
                footprint="Package_TO_SOT_SMD:SOT-23",
                pins=[Pin(num="1", name="G", func=Pin.types.INPUT),
                      Pin(num="2", name="S", func=Pin.types.PASSIVE),
                      Pin(num="3", name="D", func=Pin.types.PASSIVE)])

def DIODE(ref, value):
    return Part(tool=SKIDL, name="D", ref=ref, value=value,
                footprint="Diode_SMD:D_SOD-323",
                pins=[Pin(num="1", name="K", func=Pin.types.PASSIVE),
                      Pin(num="2", name="A", func=Pin.types.PASSIVE)])

def LED(ref, value):
    return Part(tool=SKIDL, name="LED", ref=ref, value=value,
                footprint="LED_SMD:LED_0805_2012Metric",
                pins=[Pin(num="1", name="A", func=Pin.types.PASSIVE),
                      Pin(num="2", name="K", func=Pin.types.PASSIVE)])

def SOLDER_BRIDGE_OPEN(ref, value):
    """Solder bridge — default OPEN (cut trace to bridge)."""
    return Part(tool=SKIDL, name="SolderBridge", ref=ref, value=value,
                footprint="Jumper:SolderJumper-2_P1.3mm_Open_RoundedPad1.0x1.5mm",
                pins=[Pin(num="1", name="1", func=Pin.types.PASSIVE),
                      Pin(num="2", name="2", func=Pin.types.PASSIVE)])

def SOLDER_BRIDGE_CLOSED(ref, value):
    """Solder bridge — default CLOSED (cut trace to open)."""
    return Part(tool=SKIDL, name="SolderBridge", ref=ref, value=value,
                footprint="Jumper:SolderJumper-2_P1.3mm_Bridged_RoundedPad1.0x1.5mm",
                pins=[Pin(num="1", name="1", func=Pin.types.PASSIVE),
                      Pin(num="2", name="2", func=Pin.types.PASSIVE)])

def ENCODER(ref, value):
    return Part(tool=SKIDL, name="Encoder", ref=ref, value=value,
                footprint="Rotary_Encoder:RotaryEncoder_Alps_EC12E-Switch_Vertical_H20mm",
                pins=[Pin(num="A", name="A", func=Pin.types.PASSIVE),
                      Pin(num="C", name="C", func=Pin.types.PASSIVE),
                      Pin(num="B", name="B", func=Pin.types.PASSIVE),
                      Pin(num="S1", name="S1", func=Pin.types.PASSIVE),
                      Pin(num="S2", name="S2", func=Pin.types.PASSIVE)])

# =============================================================================
# Power nets
# =============================================================================
vcc = Net("+3V3")
gnd = Net("GND")

# Signal nets
scl = Net("SCL")
sda = Net("SDA")
enc_clk = Net("ENC_CLK")
enc_dt = Net("ENC_DT")
enc_sw = Net("ENC_SW")
led_gpio = Net("LED_GPIO")
ot_tx_sig = Net("OT_TX_SIG")   # internal OT TX signal (goes to R1)
ot_rx_sig = Net("OT_RX_SIG")   # internal OT RX signal (goes to R4/U2)
ot_plus = Net("OT+")
ot_minus = Net("OT-")

# GPIO nets from connectors
gpio4 = Net("GPIO4")
gpio36 = Net("GPIO36")
uext_txd = Net("UEXT_TXD")
uext_rxd = Net("UEXT_RXD")

# =============================================================================
# J4: EXT Header — GPIO4 + GPIO36 from ESP32-POE EXT1/EXT2
# =============================================================================
j4 = make_connector("J4", "EXT_GPIO", 3, "Connector_JST:JST_XH_B3B-XH-A_1x03_P2.50mm_Vertical")

j4["P1"] += gpio4
j4["P2"] += gpio36
j4["P3"] += gnd

# =============================================================================
# J1: UEXT Connector — 2x5 female keyed socket
# =============================================================================
j1 = make_connector("J1", "UEXT", 10, "IDC:IDC-Stecker_2x05_P2.54mm_Vertical")

j1["P1"] += vcc
j1["P2"] += gnd
j1["P3"] += uext_txd
j1["P4"] += uext_rxd
j1["P5"] += scl
j1["P6"] += sda
j1["P7"] += enc_clk
j1["P8"] += led_gpio
j1["P9"] += enc_dt
j1["P10"] += enc_sw

# =============================================================================
# Solder Bridges — select OT GPIO source (cwl/cwl-1.3 only)
#
# Both routes terminate at GPIO 4 / GPIO 36 on the Olimex ESP32-POE. The bridges
# pick which physical path drives them:
#   SB3+SB4 closed (default): UEXT pins 3/4 drive OT — stacked-shield, no wires.
#   SB1+SB2 closed:           J4 (EXT GPIO header) drives OT — keeps UEXT free.
# Firmware uses `ot-ext` either way (the deprecated `ot-uext` assumed GPIO 1/3
# which aren't actually on the Olimex UEXT).
#
# Omitted entirely on cwl-diyless: the DIYless shield is wired straight to
# UEXT TXD/RXD via J6/J7 below, no bridges needed.
# =============================================================================
if VARIANT != "cwl-diyless":
    sb1 = SOLDER_BRIDGE_OPEN("SB1", "GPIO4→OT_TX")
    sb2 = SOLDER_BRIDGE_OPEN("SB2", "GPIO36→OT_RX")
    sb3 = SOLDER_BRIDGE_CLOSED("SB3", "TXD→OT_TX")
    sb4 = SOLDER_BRIDGE_CLOSED("SB4", "RXD→OT_RX")

    # SB1: GPIO4 (J4) → OT TX signal (default: open)
    gpio4 += sb1["1"]
    sb1["2"] += ot_tx_sig

    # SB2: GPIO36 (J4) → OT RX signal (default: open)
    gpio36 += sb2["1"]
    sb2["2"] += ot_rx_sig

    # SB3: UEXT TXD → OT TX signal (default: closed)
    uext_txd += sb3["1"]
    sb3["2"] += ot_tx_sig

    # SB4: UEXT RXD → OT RX signal (default: closed)
    uext_rxd += sb4["1"]
    sb4["2"] += ot_rx_sig
else:
    # cwl-diyless: wire UEXT TXD/RXD straight through to the shield's pin
    # headers (J6/J7 below). DIYless ot_tx_sig / ot_rx_sig nets are reused so
    # the rest of the netlist (J5 aux breakout) stays unchanged.
    ot_tx_sig += uext_txd
    ot_rx_sig += uext_rxd

# =============================================================================
# J5: Auxiliary Header — +3V3, GND, TXD, RXD breakout
# =============================================================================
j5 = make_connector("J5", "AUX", 4, "Connector_JST:JST_XH_B4B-XH-A_1x04_P2.50mm_Vertical")

j5["P1"] += vcc             # +3V3
j5["P2"] += gnd             # GND
j5["P3"] += uext_txd        # TXD (GPIO1)
j5["P4"] += uext_rxd        # RXD (GPIO3)

# =============================================================================
# J2: Screw Terminal — OpenTherm bus (cwl/cwl-1.3 only)
# Omitted on cwl-diyless: the DIYless shield carries its own bus terminal.
# =============================================================================
if VARIANT != "cwl-diyless":
    j2 = make_connector("J2", "OpenTherm", 2, "TerminalBlock_Phoenix:TerminalBlock_Phoenix_MKDS-1,5-2-5.08_1x02_P5.08mm_Horizontal")
    j2["P1"] += ot_plus
    j2["P2"] += ot_minus

# =============================================================================
# J3: OLED Display — 0.96" SSD1306 (always present)
# Pinout pad 1 → 4: GND, VCC, SCL, SDA
# =============================================================================
j3 = make_connector("J3", "OLED_0.96", 4, "SSD1306:128x64OLED-MountingHoles")

j3["P1"] += gnd
j3["P2"] += vcc
j3["P3"] += scl
j3["P4"] += sda

# =============================================================================
# J3B: OLED Display — 1.3" SH1106 footprint (cwl-diyless only — parallel to J3)
# Pinout pad 1 → 4: VDD, GND, SCK, SDA  (note: order differs from the 0.96")
# Populate either J3 OR J3B, not both.
# =============================================================================
if VARIANT == "cwl-diyless":
    j3b = make_connector("J3B", "OLED_1.3", 4, "SSD1306:128x64OLED-MountingHoles-Large")
    j3b["P1"] += vcc
    j3b["P2"] += gnd
    j3b["P3"] += scl
    j3b["P4"] += sda

# =============================================================================
# OpenTherm interface (cwl/cwl-1.3 only)
# Optocoupler + MOSFET + opto + zener + 1N4148 polarity-independence diodes.
# Omitted on cwl-diyless — the DIYless shield (J6/J7 below) provides the
# isolated bus interface in a single drop-in module.
# =============================================================================
if VARIANT != "cwl-diyless":
    # TX: OT_TX_SIG → R1 → U1 (opto) → Q1 (MOSFET) → OT+
    r1 = R("R1", "330")
    u1 = OPTO("U1", "LTV-817S-B")
    q1 = NMOS("Q1", "2N7002")
    r3 = R("R3", "4.7k")

    tx_led = Net("TX_LED")
    tx_gate = Net("TX_GATE")

    ot_tx_sig += r1["1"]
    r1["2"] += tx_led
    tx_led += u1["A"]
    u1["K"] += gnd
    u1["E"] += gnd
    u1["C"] += tx_gate
    tx_gate += q1["G"]
    tx_gate += r3["1"]
    r3["2"] += gnd
    q1["D"] += ot_plus
    q1["S"] += gnd

    # RX: OT+ → R2 → U2 (opto) → GPIO36
    r2 = R("R2", "680")
    r4 = R("R4", "10k")
    r5 = R("R5", "1k")
    u2 = OPTO("U2", "LTV-817S-B")
    d1 = DIODE("D1", "BZX384-C4V7")    # 4.7V zener (matches reference design)
    d2 = DIODE("D2", "1N4148WS")        # Bus protection — polarity independence
    d3 = DIODE("D3", "1N4148WS")
    d4 = DIODE("D4", "1N4148WS")
    d5 = DIODE("D5", "1N4148WS")

    rx_anode = Net("RX_ANODE")
    rx_cathode = Net("RX_CATHODE")

    ot_plus += r2["1"]
    r2["2"] += rx_anode
    rx_anode += u2["A"]
    rx_anode += d1["K"]                 # Zener cathode — clamps voltage across opto LED
    u2["K"] += rx_cathode
    rx_cathode += d1["A"]               # Zener anode
    rx_cathode += r5["1"]
    r5["2"] += ot_minus

    u2["E"] += gnd
    u2["C"] += ot_rx_sig
    ot_rx_sig += r4["1"]
    r4["2"] += vcc

    # Full bridge protection — makes OT bus polarity-independent
    # Forward path:  OT+ → D2(A→K) → +3V3 clamp
    #                GND → D3(A→K) → OT-
    # Reverse path:  OT- → D4(A→K) → +3V3 clamp
    #                GND → D5(A→K) → OT+
    d2["A"] += ot_plus
    d2["K"] += vcc
    d3["A"] += gnd
    d3["K"] += ot_minus
    d4["A"] += ot_minus
    d4["K"] += vcc
    d5["A"] += gnd
    d5["K"] += ot_plus

# =============================================================================
# J6 + J7: DIYless D1-mini OpenTherm shield direct-solder pads (cwl-diyless only)
#
# The shield's underside has the standard D1 mini 2×8 pin header layout. Two
# 1×8 through-hole rows on this PCB receive the shield's pins; the user
# solders the shield directly. Only 4 of the 16 pins are wired here:
#
#   J6 (left side, 1×8): pins 1..7 = RST/A0/D0/D5/D6/D7/D8 (NC), pin 8 = 3V3
#   J7 (right side, 1×8): pin 1 = TX (NC), pin 2 = RX (NC),
#                         pin 3 = D1 (GPIO 5)  → OT TX from shield
#                         pin 4 = D2 (GPIO 4)  → OT RX into shield
#                         pin 5 = D3 (NC), pin 6 = D4 (NC),
#                         pin 7 = GND, pin 8 = 5V (NC — wire externally if your
#                                                 DIYless revision needs it)
#
# Row spacing in PCB layout: 22.86 mm (0.9") center-to-center (D1 mini standard).
# =============================================================================
if VARIANT == "cwl-diyless":
    j6 = make_connector("J6", "D1mini_L", 8, "Connector_PinHeader_2.54mm:PinHeader_1x08_P2.54mm_Vertical")
    j6["P8"] += vcc  # 3V3

    j7 = make_connector("J7", "D1mini_R", 8, "Connector_PinHeader_2.54mm:PinHeader_1x08_P2.54mm_Vertical")
    j7["P3"] += ot_tx_sig  # D1 (GPIO 5) — MCU → shield → OT bus
    j7["P4"] += ot_rx_sig  # D2 (GPIO 4) — OT bus → shield → MCU
    j7["P7"] += gnd        # GND

# =============================================================================
# J8: UEXT debug breakout (cwl-diyless only) — all 10 UEXT signals on a 1×10
# pin header for probing / scope / logic analyser hook-up. Mirrors J1.
# =============================================================================
if VARIANT == "cwl-diyless":
    j8 = make_connector("J8", "UEXT_DBG", 10, "Connector_PinHeader_2.54mm:PinHeader_1x10_P2.54mm_Vertical")
    j8["P1"] += vcc
    j8["P2"] += gnd
    j8["P3"] += uext_txd
    j8["P4"] += uext_rxd
    j8["P5"] += scl
    j8["P6"] += sda
    j8["P7"] += enc_clk
    j8["P8"] += led_gpio
    j8["P9"] += enc_dt
    j8["P10"] += enc_sw

# =============================================================================
# SW1: Encoder — Alps EC12E24204A9
# =============================================================================
sw1 = ENCODER("SW1", "EC12E24204A9")
r6 = R("R6", "10k")
r7 = R("R7", "10k")

sw1["A"] += enc_clk
sw1["B"] += enc_dt
sw1["C"] += gnd
sw1["S1"] += enc_sw
sw1["S2"] += gnd

enc_clk += r6["1"]
r6["2"] += vcc
enc_dt += r7["1"]
r7["2"] += vcc

# =============================================================================
# LED1: Status LED
# =============================================================================
r8 = R("R8", "1k")
led1 = LED("LED1", "Green")

led_anode = Net("LED_ANODE")
led_gpio += r8["1"]
r8["2"] += led_anode
led_anode += led1["A"]
led1["K"] += gnd

# =============================================================================
# Mounting Holes (M3, connected to GND)
# =============================================================================
for i in range(1, 5):
    mh = Part(tool=SKIDL, name="MountingHole", ref=f"MH{i}", value="M3",
              footprint="MountingHole:MountingHole_3.2mm_M3_Pad_Via",
              pins=[Pin(num="1", name="1", func=Pin.types.PASSIVE)])
    mh["1"] += gnd

# =============================================================================
# Generate
# =============================================================================
if __name__ == "__main__":
    out_file = f"wolf-cwl-shield-{VARIANT}.net" if VARIANT != "cwl" else "wolf-cwl-shield.net"
    generate_netlist(file_=out_file)

    # Print circuit summary for review
    print("\n" + "=" * 70)
    print(f"CIRCUIT SUMMARY — Wolf CWL OpenTherm Shield  [variant: {VARIANT}]")
    print("=" * 70)

    print("\nCOMPONENTS:")
    print("-" * 50)
    for part in sorted(default_circuit.parts, key=lambda p: p.ref):
        print(f"  {part.ref:6s}  {part.value:20s}  {part.footprint}")

    print("\nNET CONNECTIONS:")
    print("-" * 50)
    for net in sorted(default_circuit.nets, key=lambda n: n.name):
        if net.name.startswith("N$"):
            continue  # skip unnamed nets
        pins = [f"{p.part.ref}.{p.num}" for p in net.pins]
        if len(pins) >= 2:
            print(f"  {net.name:20s}  {', '.join(sorted(pins))}")

    print("\nUNCONNECTED PINS:")
    print("-" * 50)
    found_unconnected = False
    for part in sorted(default_circuit.parts, key=lambda p: p.ref):
        for pin in part.pins:
            if not pin.nets or all(n.name.startswith("N$") and len(n.pins) <= 1 for n in pin.nets):
                print(f"  {part.ref}.{pin.num} ({pin.name}) — NOT CONNECTED")
                found_unconnected = True
    if not found_unconnected:
        print("  None")

    print("\n" + "=" * 70)
    print(f"Generated: {out_file}")
    print("\nImport into KiCad:")
    print(f"  1. Open KiCad → open {VARIANT}/cwl.kicad_pro (or create the project)")
    print( "  2. Open PCB Editor")
    print(f"  3. File → Import Netlist → {out_file}")
    print( "  4. All footprints are pre-assigned — place and route")
