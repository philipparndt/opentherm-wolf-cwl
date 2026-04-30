"""
Wolf CWL OpenTherm Shield — KiCad netlist generator using SKiDL.

Run with: python generate_schematic.py
Output: wolf-cwl-shield.net (KiCad netlist)
"""

import warnings
warnings.filterwarnings("ignore")

from skidl import *

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
                footprint="Package_SO:SOP-4_3.8x4.1mm_P2.54mm",
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
# Solder Bridges — select OT GPIO source
#
# Option A (default): Bridge SB3+SB4, leave SB1+SB2 open
#   → OT uses UEXT TXD(GPIO1)/RXD(GPIO3) — no extra wires needed
#   → Firmware feature: ot-uext (default)
#
# Option B (fallback): Bridge SB1+SB2, leave SB3+SB4 open
#   → OT uses GPIO4/GPIO36 via J4 (EXT header wires)
#   → Keeps serial debug on UEXT TXD/RXD
#   → Firmware feature: ot-ext
# =============================================================================
sb1 = SOLDER_BRIDGE_OPEN("SB1", "GPIO4→OT_TX")
sb2 = SOLDER_BRIDGE_OPEN("SB2", "GPIO36→OT_RX")
sb3 = SOLDER_BRIDGE_CLOSED("SB3", "TXD→OT_TX")
sb4 = SOLDER_BRIDGE_CLOSED("SB4", "RXD→OT_RX")

# SB1: GPIO4 (J4) → OT TX signal (default: closed)
gpio4 += sb1["1"]
sb1["2"] += ot_tx_sig

# SB2: GPIO36 (J4) → OT RX signal (default: closed)
gpio36 += sb2["1"]
sb2["2"] += ot_rx_sig

# SB3: UEXT TXD → OT TX signal (default: open)
uext_txd += sb3["1"]
sb3["2"] += ot_tx_sig

# SB4: UEXT RXD → OT RX signal (default: open)
uext_rxd += sb4["1"]
sb4["2"] += ot_rx_sig

# =============================================================================
# J5: Auxiliary Header — +3V3, GND, TXD, RXD breakout
# =============================================================================
j5 = make_connector("J5", "AUX", 4, "Connector_JST:JST_XH_B4B-XH-A_1x04_P2.50mm_Vertical")

j5["P1"] += vcc             # +3V3
j5["P2"] += gnd             # GND
j5["P3"] += uext_txd        # TXD (GPIO1)
j5["P4"] += uext_rxd        # RXD (GPIO3)

# =============================================================================
# J2: Screw Terminal — OpenTherm bus
# =============================================================================
j2 = make_connector("J2", "OpenTherm", 2, "TerminalBlock_Phoenix:TerminalBlock_Phoenix_MKDS-1,5-2-5.08_1x02_P5.08mm_Horizontal")

j2["P1"] += ot_plus
j2["P2"] += ot_minus

# =============================================================================
# J3: OLED Display — direct solder pads
# =============================================================================
j3 = make_connector("J3", "OLED_SH1106", 4, "SSD1306:128x64OLED-MountingHoles")

j3["P1"] += gnd
j3["P2"] += vcc
j3["P3"] += scl
j3["P4"] += sda

# =============================================================================
# OpenTherm TX: OT_TX_SIG → R1 → U1 (opto) → Q1 (MOSFET) → OT+
# =============================================================================
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

# =============================================================================
# OpenTherm RX: OT+ → R2 → U2 (opto) → GPIO36
# =============================================================================
r2 = R("R2", "680")
r4 = R("R4", "10k")
r5 = R("R5", "1k")
u2 = OPTO("U2", "LTV-817S-B")
d1 = DIODE("D1", "BZX384-C3V3")
d2 = DIODE("D2", "1N4148WS")
d3 = DIODE("D3", "1N4148WS")

rx_anode = Net("RX_ANODE")
rx_cathode = Net("RX_CATHODE")

ot_plus += r2["1"]
r2["2"] += rx_anode
rx_anode += u2["A"]
rx_anode += d1["K"]
u2["K"] += rx_cathode
rx_cathode += d1["A"]
rx_cathode += r5["1"]
r5["2"] += ot_minus

u2["E"] += gnd
u2["C"] += ot_rx_sig
ot_rx_sig += r4["1"]
r4["2"] += vcc

d2["A"] += ot_plus
d2["K"] += vcc
d3["A"] += gnd
d3["K"] += ot_minus

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
    generate_netlist(file_="wolf-cwl-shield.net")

    # Print circuit summary for review
    print("\n" + "=" * 70)
    print("CIRCUIT SUMMARY — Wolf CWL OpenTherm Shield")
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
    print(f"Generated: wolf-cwl-shield.net")
    print("\nImport into KiCad:")
    print("  1. Open KiCad → New Project")
    print("  2. Open PCB Editor")
    print("  3. File → Import Netlist → wolf-cwl-shield.net")
    print("  4. All footprints are pre-assigned — place and route")
