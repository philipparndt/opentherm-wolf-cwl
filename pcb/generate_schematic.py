"""
Wolf CWL OpenTherm Shield — KiCad netlist generator using SKiDL.

Run with: python generate_schematic.py [--variant=NAME]
Or:       VARIANT=cwl-diyless python generate_schematic.py

Variants:
  cwl          (default) — 0.96" OLED, on-board OT optocoupler/MOSFET circuit (v1.x — buggy, see DEBUG-NOTES.md)
  cwl-1.3      — same v1.x circuit, 1.3" OLED footprint
  cwl-2.0      — 0.96" OLED, Melnyk-topology OT front-end (recommended for new builds)
  cwl-diyless  — no on-board OT (use a DIYless D1 mini OT shield instead),
                 both 0.96" and 1.3" OLED footprints (populate one)

Output: wolf-cwl-shield[-VARIANT].net (KiCad netlist) and .svg (visual review)
"""

import os
import sys
import warnings
warnings.filterwarnings("ignore")

from skidl import *

# Suppress SKiDL warnings about missing KiCad libs
import logging
logging.getLogger("skidl").setLevel(logging.ERROR)

# Variant selection — env var or `--variant=` CLI flag
VARIANT = os.environ.get("VARIANT", "cwl")
for arg in sys.argv[1:]:
    if arg.startswith("--variant="):
        VARIANT = arg.split("=", 1)[1]
if VARIANT not in ("cwl", "cwl-1.3", "cwl-2.0", "cwl-diyless"):
    raise SystemExit(f"Unknown VARIANT: {VARIANT!r} (expected cwl, cwl-1.3, cwl-2.0, or cwl-diyless)")

# Use real KiCad library symbols so generate_schematic() emits a proper .kicad_sch.
# (The earlier tool=SKIDL approach worked for netlist export but produced no
# visible symbols on the schematic side — and skipping the schematic shipped
# multiple silent topology bugs to fab. See DEBUG-NOTES.md.)
set_default_tool(KICAD)

_KICAD_SYM_DEFAULT = "/Applications/KiCad/KiCad.app/Contents/SharedSupport/symbols"
_kicad_sym = os.environ.get("KICAD_SYMBOL_DIR", _KICAD_SYM_DEFAULT)
if _kicad_sym and _kicad_sym not in lib_search_paths[KICAD]:
    lib_search_paths[KICAD].append(_kicad_sym)

# =============================================================================
# Part factories — wrap real KiCad library symbols.
#
# Why real libraries: SKiDL's generate_schematic() emits a .kicad_sch using
# the symbol's graphic definition. Custom tool=SKIDL parts have no graphic,
# so the output is empty / unusable. By referencing the bundled KiCad symbols
# (Device:R, Transistor_BJT:BC858, Isolator:PC817, ...) we get a real
# schematic we can open in eeschema and rearrange.
#
# Pin aliases: existing call-sites use friendly names like u1["A"], j2["P1"].
# Real KiCad symbols sometimes use only pin numbers ("1") or different names
# ("Pin_1"), so each factory adds the aliases the rest of the code expects.
# =============================================================================

def _alias_connector(p, pin_count):
    """Add P1..Pn aliases so existing j["P1"] syntax still works."""
    for i in range(1, pin_count + 1):
        p[i].aliases += f"P{i}"
    return p

def make_connector(ref, value, pin_count, footprint):
    # Pick the connector symbol that matches the physical shape. We disambiguate
    # 2x05 vs 1x10 (both 10 pins) by sniffing the footprint name.
    if "2x05" in footprint or "02x05" in footprint:
        lib, sym = "Connector_Generic", "Conn_02x05_Odd_Even"
    elif "2x08" in footprint or "02x08" in footprint:
        lib, sym = "Connector_Generic", "Conn_02x08_Odd_Even"
    elif pin_count <= 4:
        lib, sym = "Connector", f"Conn_01x0{pin_count}_Pin"
    elif pin_count == 8:
        lib, sym = "Connector", "Conn_01x08_Pin"
    elif pin_count == 10:
        lib, sym = "Connector", "Conn_01x10_Pin"
    else:
        raise ValueError(f"No connector symbol mapping for pin_count={pin_count}, footprint={footprint}")
    p = Part(lib, sym, ref=ref, value=value, footprint=footprint)
    return _alias_connector(p, pin_count)

def R(ref, value):
    return Part("Device", "R", ref=ref, value=value,
                footprint="Resistor_SMD:R_0603_1608Metric")

def OPTO(ref, value):
    # PC817 / LTV-817 SOIC-4: 1=Anode, 2=Cathode, 3=Emitter, 4=Collector
    # KiCad's Isolator:PC817 leaves pin names empty — alias them so the rest
    # of the code can keep using u["A"], u["K"], u["E"], u["C"].
    p = Part("Isolator", "PC817", ref=ref, value=value,
             footprint="Package_SO:SOP-4_7.5x4.1mm_P2.54mm")
    p[1].aliases += "A"
    p[2].aliases += "K"
    p[3].aliases += "E"
    p[4].aliases += "C"
    return p

def NMOS(ref, value):
    # Transistor_FET:2N7002 extends Q_NMOS_GSD → pin names G/S/D.
    return Part("Transistor_FET", "2N7002", ref=ref, value=value,
                footprint="Package_TO_SOT_SMD:SOT-23")

def PNP(ref, value):
    # Transistor_BJT:BC858 extends Q_PNP_BEC → pin names B/E/C.
    return Part("Transistor_BJT", "BC858", ref=ref, value=value,
                footprint="Package_TO_SOT_SMD:SOT-23")

def DIODE(ref, value):
    # Footprint matches the actual SMD package of the part number:
    #   BZT52C*    → SOD-123    (0.5W zener series, Diodes Inc / Vishay / NXP)
    #   BZX384*    → SOD-323    (smaller 0.2W zener series)
    #   1N4148W    → SOD-123    | 1N4148WT → SOD-523
    #   1N4148WS   → SOD-323F   (flat-lead variant — matches the stock on hand
    #                            for this build; vendors also use the plain
    #                            D_SOD-323 footprint, but pad spacing differs
    #                            by 0.1 mm which hurts reflow self-alignment)
    # Schematic symbol: D_Zener for zener part numbers, plain D otherwise.
    sym = "D_Zener" if value.startswith("BZ") else "D"
    if value.startswith("BZT52") or value.endswith("1N4148W"):
        fp = "Diode_SMD:D_SOD-123"
    elif value.startswith("BZX85"):
        fp = "Diode_SMD:D_SOD-80"
    elif value == "1N4148WS":
        fp = "Diode_SMD:D_SOD-323F"
    else:
        fp = "Diode_SMD:D_SOD-323"  # BZX384 and other standard SOD-323 parts
    return Part("Device", sym, ref=ref, value=value, footprint=fp)

def LED(ref, value):
    return Part("Device", "LED", ref=ref, value=value,
                footprint="LED_SMD:LED_0805_2012Metric")

def SOLDER_BRIDGE_OPEN(ref, value):
    """Solder bridge — default OPEN (cut trace to bridge)."""
    return Part("Jumper", "SolderJumper_2_Open", ref=ref, value=value,
                footprint="Jumper:SolderJumper-2_P1.3mm_Open_RoundedPad1.0x1.5mm")

def SOLDER_BRIDGE_CLOSED(ref, value):
    """Solder bridge — default CLOSED (cut trace to open)."""
    return Part("Jumper", "SolderJumper_2_Bridged", ref=ref, value=value,
                footprint="Jumper:SolderJumper-2_P1.3mm_Bridged_RoundedPad1.0x1.5mm")

def ENCODER(ref, value):
    # Device:RotaryEncoder_Switch — pins A, B, C, S1, S2 (already match call-sites).
    return Part("Device", "RotaryEncoder_Switch", ref=ref, value=value,
                footprint="Rotary_Encoder:RotaryEncoder_Alps_EC12E-Switch_Vertical_H20mm")

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
# Omitted on cwl-diyless: the DIYless shield doesn't expose these GPIOs.
# =============================================================================
if VARIANT != "cwl-diyless":
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
    # headers (J6/J7 below). No solder bridges, no GPIO4/GPIO36 routing.
    ot_tx_sig += uext_txd
    ot_rx_sig += uext_rxd

# =============================================================================
# J5: Auxiliary Header — +3V3, GND, TXD, RXD breakout
# Omitted on cwl-diyless: J8 (UEXT_DBG) already breaks out these signals.
# =============================================================================
if VARIANT != "cwl-diyless":
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
# J3: OLED Display
#   cwl / cwl-1.3   : 0.96" SH1106 only, footprint 128x64OLED-MountingHoles
#                     Pinout pad 1→4: GND, VCC, SCL, SDA
#   cwl-2.0         : combined footprint that accepts EITHER a 0.96" or a
#                     1.3" SH1106 module. 8 pads total — pads 1–4 for the
#                     1.3" header, pads 5–8 for the 0.96" header. Each pin
#                     wires to its module's required net; populating one
#                     module leaves the other pad set unused. 3D model is
#                     the 1.3" assembly (worst-case for case design).
#   cwl-diyless     : 0.96" via J3 (below) AND a separate J3B for the 1.3",
#                     placed at a different XY on the board (legacy layout).
# =============================================================================
if VARIANT == "cwl-2.0":
    # Combined footprint has 8 physical pads but only 4 unique pin numbers
    # (each appearing twice — once at the 1.3" header position, once at the
    # 0.96" header position). Schematic symbol is a clean 4-pin connector.
    j3 = make_connector("J3", "OLED_0.96+1.3", 4, "SSD1306:128x64OLED-MountingHoles-Combined")
    j3["P1"] += vcc
    j3["P2"] += gnd
    j3["P3"] += scl
    j3["P4"] += sda
else:
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
# OpenTherm interface — variant-specific
#
#   cwl / cwl-1.3   : legacy v1.x circuit. NMOS sink + shunt-diode "polarity
#                     protection". Has documented bugs — see DEBUG-NOTES.md.
#                     Kept here so the existing KiCad projects still build.
#   cwl-2.0         : Melnyk-topology front-end (recommended). See block below.
#   cwl-diyless     : no on-board OT; DIYless shield (J6) handles isolation.
# =============================================================================

if VARIANT in ("cwl", "cwl-1.3"):
    # --- Legacy v1.x — KNOWN BUGGY, kept for compatibility with existing PCBs ---
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

elif VARIANT == "cwl-2.0":
    # --- Melnyk topology — https://ihormelnyk.com/opentherm_adapter ---
    # Verified pin-for-pin against the published schematic. Replaces the
    # three classes of bug in cwl/cwl-1.3:
    #   - Four shunt diodes  → real Graetz bridge (D1..D4)
    #   - Inverting NMOS TX  → non-inverting PNP current sink (Q1 BC858A)
    #   - No bus-side clamps → D5/D6/D7 zeners protect transistor terminals
    #
    # Bus side (galvanically isolated from MCU GND through U1/U2 PC817 optos):
    #   - D1..D4 form a Graetz bridge → polarity-independent BUS_HI / BUS_LO
    #   - Q1 (BC858A PNP): emitter at BUS_HI; base pulled high by R1 (330)
    #     to BUS_HI so idle = Q1 OFF. The R1↔Q1.B node also carries U1.C
    #     and D6.K — D6 (15V zener) clamps it to BUS_LO+15V.
    #   - Q1.C → R2 (220) → "U2-LED node". This same node carries U2.A and
    #     R3.top. R3 (100) and U2 LED are in parallel between U2-LED node
    #     and BUS_LO. D5 (4V7) clamps Q1.C to BUS_LO+4.7V.
    #   - U1 photo emitter is clamped by D7 (4V3) to BUS_LO+4.3V.
    #
    # When U1 LED conducts (TX HIGH from MCU):
    #   U1 photo links Q1.B (≈ BUS_LO+15V via D6) down to U1.E (≈ BUS_LO+4.3V
    #   via D7). Q1.B drops well below Q1.E (BUS_HI), Q1 turns ON, current
    #   sinks BUS_HI → Q1 → R2 → (R3 ∥ U2 LED) → BUS_LO. The U2 LED conducts
    #   as a side effect of the TX state, so the MCU's RX line also tracks
    #   what we're transmitting. The slave's response modulates bus voltage,
    #   which changes current through R2/R3/U2 LED; the U2 photo decodes
    #   that modulation back to the MCU.
    #
    # MCU side:
    #   TX: OT_TX_SIG → R4 (330) → U1 LED anode; U1 LED cathode → GND.
    #       Non-inverting: GPIO HIGH ⇒ U1 LED on ⇒ Q1 on ⇒ master sinks
    #       high current. Matches ihormelnyk/opentherm_library defaults.
    #   RX: U2 in emitter-follower configuration — collector at +3V3,
    #       emitter through R5 (1.5k) to GND, OT_RX_SIG taken from emitter.
    #       Bus carrying current ⇒ U2 photo on ⇒ OT_RX_SIG HIGH.

    bus_hi   = Net("BUS_HI")
    bus_lo   = Net("BUS_LO")
    q1_base  = Net("Q1_BASE")   # D6.K, Q1.B, R1.right, U1.C
    q1_coll  = Net("Q1_COLL")   # D5.K, Q1.C, R2.left
    u2_led_a = Net("U2_LED_A")  # R2.right, R3.top, U2.A
    u1_emit  = Net("U1_PHOTO_E") # D7.K, U1.E

    # Graetz bridge — polarity-independent rectification of the OT bus
    d1 = DIODE("D1", "1N4148WS")
    d2 = DIODE("D2", "1N4148WS")
    d3 = DIODE("D3", "1N4148WS")
    d4 = DIODE("D4", "1N4148WS")
    d1["A"] += ot_plus;  d1["K"] += bus_hi
    d2["A"] += ot_minus; d2["K"] += bus_hi
    d3["A"] += bus_lo;   d3["K"] += ot_plus
    d4["A"] += bus_lo;   d4["K"] += ot_minus

    # Bus-side clamps (all zener anodes at BUS_LO — vertical column in Melnyk)
    d5 = DIODE("D5", "BZT52C4V7")  # Q1 collector clamp
    d6 = DIODE("D6", "BZT52C15")   # Q1 base / U1.C clamp
    d7 = DIODE("D7", "BZT52C4V3")  # U1 photo emitter clamp
    d5["K"] += q1_coll;  d5["A"] += bus_lo
    d6["K"] += q1_base;  d6["A"] += bus_lo
    d7["K"] += u1_emit;  d7["A"] += bus_lo

    # Q1 PNP current sink (BC858A)
    q1 = PNP("Q1", "BC858A")
    r1 = R("R1", "330")        # Q1 base pull-up to BUS_HI
    r2 = R("R2", "220")        # Q1 collector → U2-LED node
    r3 = R("R3", "100")        # U2-LED node → BUS_LO (parallel with U2 LED)

    bus_hi  += q1["E"]
    bus_hi  += r1["1"]
    r1["2"] += q1_base
    q1_base += q1["B"]
    q1_coll += q1["C"]
    q1_coll += r2["1"]
    r2["2"]  += u2_led_a
    u2_led_a += r3["1"]
    r3["2"]  += bus_lo

    # U1 — TX opto. LED driven from OT_TX_SIG; photo couples Q1.B to U1.E.
    u1 = OPTO("U1", "PC817")
    r4 = R("R4", "330")
    ot_tx_sig += r4["1"]
    r4["2"]   += u1["A"]
    u1["K"]   += gnd
    u1["C"]   += q1_base
    u1["E"]   += u1_emit

    # U2 — RX opto. LED in series in the master's current-sink path; photo
    # in emitter-follower on MCU side.
    u2 = OPTO("U2", "PC817")
    r5 = R("R5", "1.5k")
    u2_led_a += u2["A"]
    u2["K"]  += bus_lo
    u2["C"]  += vcc
    u2["E"]  += ot_rx_sig
    ot_rx_sig += r5["1"]
    r5["2"]  += gnd

# =============================================================================
# J6: DIYless D1-mini OpenTherm shield direct-solder pads (cwl-diyless only)
#
# Single 2x8 footprint with the D1 mini's 22.86 mm (0.9") row spacing baked
# in, so both rows are locked together. Pin numbering:
#
#   Left column  (pins 1..8, top→bottom):
#     1=RST, 2=A0, 3=D0, 4=D5, 5=D6, 6=D7, 7=D8, 8=3V3
#   Right column (pins 9..16, top→bottom):
#     9=TX, 10=RX, 11=D1 (GPIO5), 12=D2 (GPIO4),
#     13=D3, 14=D4, 15=GND, 16=5V
#
# Only 4 of 16 pins are wired (3V3, D1, D2, GND); the rest are NC pads that
# physically support the shield. Wire pin 16 (5V) externally if your DIYless
# revision needs it.
# =============================================================================
if VARIANT == "cwl-diyless":
    j6 = make_connector("J6", "D1mini", 16, "D1mini:D1_Mini_2x08_P2.54mm")
    j6["P8"]  += vcc       # 3V3
    j6["P11"] += ot_tx_sig # D1 (GPIO 5) — MCU → shield → OT bus
    j6["P12"] += ot_rx_sig # D2 (GPIO 4) — OT bus → shield → MCU
    j6["P15"] += gnd       # GND

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
    # MountingHole_Pad has one electrical pin (the pad) — wired to GND.
    mh = Part("Mechanical", "MountingHole_Pad", ref=f"MH{i}", value="M3",
              footprint="MountingHole:MountingHole_3.2mm_M3_Pad_Via")
    mh["1"] += gnd

# =============================================================================
# Generate
# =============================================================================
if __name__ == "__main__":
    base_name = "wolf-cwl-shield" if VARIANT == "cwl" else f"wolf-cwl-shield-{VARIANT}"
    out_file = f"{base_name}.net"
    generate_netlist(file_=out_file)

    # Emit a real KiCad schematic (.kicad_sch) for visual review. Auto-placement
    # is rough — that's fine, open it in eeschema and arrange by hand. The point
    # is to surface topology bugs (wrong-polarity diodes, inverted drivers,
    # mis-wired pull-ups) that are invisible in netlist form. See DEBUG-NOTES.md.
    sch_generated = False
    try:
        # flatness=1.0   : single flat sheet (no hierarchy, easier to scan).
        # auto_stub=True : when SKiDL's experimental router can't lay out a net,
        #                  fall back to labels-only — the component is still
        #                  visible with a named stub, which is enough for the
        #                  topology review we actually care about.
        generate_schematic(top_name=base_name, flatness=1.0, auto_stub=True)
        sch_generated = True
    except Exception as e:
        sch_error = f"{type(e).__name__}: {e}"

    # Optional: netlistsvg (block-diagram view, ugly but auto-routed).
    svg_generated = False
    try:
        generate_svg(file_=base_name)
        svg_generated = True
    except Exception as e:
        svg_error = e

    # Print circuit summary for review
    print("\n" + "=" * 70)
    print(f"CIRCUIT SUMMARY — Wolf CWL OpenTherm Shield  [variant: {VARIANT}]")
    print("=" * 70)

    print("\nCOMPONENTS:")
    print("-" * 50)
    for part in sorted(default_circuit.parts, key=lambda p: p.ref):
        fp = getattr(part, "footprint", "") or ""
        print(f"  {part.ref:6s}  {part.value:20s}  {fp}")

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
    if sch_generated:
        print(f"Generated: {base_name}.kicad_sch  (open in eeschema to review topology)")
    else:
        print(f"Schematic skipped: {sch_error}")
    if svg_generated:
        print(f"Generated: {base_name}.svg  (netlistsvg block diagram)")
    else:
        print(f"SVG skipped: {svg_error}")
        print("  Install with: npm install -g netlistsvg")
    print("\nReview workflow:")
    print(f"  1. Open {base_name}.kicad_sch in eeschema, sanity-check the topology")
    print( "  2. Rearrange parts for readability (auto-placement is rough)")
    print(f"  3. Open {VARIANT}/cwl.kicad_pro (or create it), PCB Editor")
    print(f"  4. File → Import Netlist → {out_file}")
    print( "  5. All footprints are pre-assigned — place and route")
