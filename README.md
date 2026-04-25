# Wolf CWL OpenTherm Decoder

Decoder tool for analyzing OpenTherm protocol communication between a BM remote control and a Wolf CWL ventilation/heat-recovery unit. Built to reverse-engineer the protocol for building a custom ESP32-based controller.

## Usage

```bash
# Default: human-readable deduplicated output
decode <file.sal|file.csv>

# Verbose: raw hex, parity, bit-timing analysis
decode <file> --verbose

# Diff: compare two captures side by side, highlights changes
decode diff <baseline.sal> <changed.sal>
```

Supports Saleae Logic 2 captures (`.sal`) and CSV exports (`.csv`). SAL files are significantly faster to process.

## Findings

### Control Summary

Everything needed to control the Wolf CWL via OpenTherm:

| Function | Method |
|----------|--------|
| **Ventilation level** | Write-Data to ID 71, value 0-3 |
| **Bypass open** | Set bit 1 in HI byte of ID 70 Read-Data request |
| **Bypass close** | Clear bit 1 in HI byte of ID 70 Read-Data request |
| **Filter reset** | Set bit 4 in HI byte of ID 70 Read-Data request (reset filter timer after replacement) |
| **Read temperatures** | Read-Data ID 80 (supply inlet), ID 82 (exhaust inlet) |
| **Read ventilation %** | Read-Data ID 77 |
| **Read bypass state** | Check `ventilation` bit (bit 1 HI byte) in ID 70 response |
| **Read fault status** | Check `fault` bit (bit 0 HI byte) in ID 70 response |
| **Read filter status** | Check `filter` bit (bit 4 HI byte) in ID 70 response (on = filter needs replacement) |

### Supported OpenTherm Data IDs

The Wolf CWL (tested with CWL Excellent, BM remote control as master) responds to the following data IDs:

| Data ID | Name | Type | Direction | Description |
|---------|------|------|-----------|-------------|
| 2 | Master configuration | config/member-id | Write | Master flags=0x00, member-id=18 |
| 70 | Status V/H | flag8 | Read | Bidirectional: master sends control flags, slave returns status |
| 71 | Control setpoint V/H | u8 | Write | Ventilation level (0-3) |
| 72 | Fault flags/code V/H | flag8 | Read | Fault flags + OEM fault code |
| 74 | Config/member-ID V/H | config/member-id | Read | Slave flags=0x07, member-id=0 |
| 77 | Relative ventilation | u8 (%) | Read | Current ventilation percentage |
| 80 | Supply inlet temperature | f8.8 (°C) | Read | Fresh air intake temperature |
| 82 | Exhaust inlet temperature | f8.8 (°C) | Read | Room exhaust temperature |
| 89 | TSP index/value V/H | tsp | Read | Transparent Slave Parameters (see below) |
| 126 | Master product version | version | Write | type=18 version=2 |
| 127 | Slave product version | version | Read | type=0 version=0 (CWL does not respond) |

### Status V/H (ID 70) — Bidirectional Control

The Status V/H exchange carries control flags in **both directions**. This is the key mechanism for bypass control.

**Master → Slave (HI byte of Read-Data request):**

| Bit | Name | Description |
|-----|------|-------------|
| 0 | ch | Central heating / ventilation enable |
| 1 | **bypass** | **Bypass damper control: 1=open, 0=closed** |
| 2 | cooling | Cooling enable (if cooling register installed) |
| 3 | dhw | DHW enable |
| 4 | filter | Filter timer reset (set briefly after filter replacement) |
| 5 | diag | Diagnostics request |

**Slave → Master (HI byte of Read-Ack response):**

| Bit | Name | Description |
|-----|------|-------------|
| 0 | fault | Fault indication (toggles briefly during mode transitions) |
| 1 | ventilation | Ventilation/bypass active (confirms bypass state) |
| 2 | cooling | Cooling active |
| 3 | dhw | DHW active |
| 4 | filter | Filter needs replacement (1 = replace filter) |
| 5 | diag | Diagnostic event active |

The bypass is controlled by setting/clearing **bit 1 of the HI byte** in every Status V/H request. The slave confirms the state via the `ventilation` bit in the response.

### Ventilation Level Control

Ventilation level is controlled via **Data ID 71** (Control setpoint V/H). The mapping is:

| Setpoint value | Mode | Relative ventilation (ID 77) |
|---------------|------|------------------------------|
| 0 | Off | 0% |
| 1 | Reduced | 51% |
| 2 | Normal | 67% |
| 3 | Party | 100% |

The `fault` bit in Status V/H (ID 70) may toggle briefly during mode transitions. This appears to be a transient state, not an actual fault indication.

### TSP Register Map

The BM remote cycles through TSP registers via Data ID 89, reading one index per polling cycle (~11 seconds per cycle). A full TSP scan takes approximately 5-6 minutes (31 indices).

TSP values did **not** change when switching ventilation levels or toggling bypass. Ventilation is controlled via ID 71, bypass via ID 70. TSPs hold configuration and live diagnostic data.

Many TSP indices come in even/odd pairs where the odd index is 0, suggesting 16-bit values split across two bytes (even=LO, odd=HI).

The following TSP indices have been observed:

#### Airflow Configuration (likely m³/h)

| TSP Index | Value | Interpretation |
|-----------|-------|----------------|
| 0, 1 | 100, 0 | Airflow rate for level 1 (Reduced): **100 m³/h** |
| 2, 3 | 130, 0 | Airflow rate for level 2 (Normal): **130 m³/h** |
| 4, 5 | 195, 0 | Airflow rate for level 3 (Party): **195 m³/h** |
| 6 | 20 | Minimum airflow or temperature threshold |
| 11 | 100 | Max ventilation % or reference value |

**Note:** These airflow values are unconfirmed — verify by changing the airflow settings on the BM and re-capturing.

#### Configuration Parameters

| TSP Index | Value | Notes |
|-----------|-------|-------|
| 7 | 44 | Unknown config |
| 17 | 1 | Config flag |
| 18 | 1 | Config flag |
| 19 | 2 | Config flag (possibly default ventilation level) |
| 20 | 0 | Config flag |
| 23, 24 | 1, 0 | Config flag |
| 48, 49 | 193, 0 | Device config flags (0xC1) |
| 52, 53 | 100-130, 0 | Possibly duplicate airflow config |
| 54 | 1 | Config flag |
| 66 | 196-255 | 0xFF likely means disabled/not configured |
| 67, 68 | 0, 0 | Unused |

#### Live Values (change between or within captures)

| TSP Index | Value range | Notes |
|-----------|-------------|-------|
| 55 | 114-118 | Varies between captures — possibly a sensor reading |
| 56 | 121 | Possibly a sensor reading |
| 60 | 0-130 | Changes **within** captures — likely bypass damper position or runtime counter |
| 61 | 0 | |
| 62 | 100-113 | Varies between captures |
| 63 | 0 | |
| 64 | 20-215 | Varies significantly — live sensor reading |
| 65 | 0-1 | |

### Data IDs to Probe

The BM remote only polls a subset of the available V/H data IDs. The following IDs are defined in the OpenTherm V/H spec and may be supported by the CWL, but have not been tested yet. Probe these with the ESP32 firmware to discover additional sensors:

| Data ID | Name | Type | Notes |
|---------|------|------|-------|
| 78 | Relative humidity exhaust | u8 (%) | Humidity sensor (if installed) |
| 79 | CO2 level exhaust | u16 (ppm) | CO2/VOC sensor (if installed) |
| 81 | Supply outlet temperature | f8.8 (°C) | Temperature after heat recovery |
| 83 | Exhaust outlet temperature | f8.8 (°C) | Temperature after heat recovery |
| 84 | Exhaust fan speed | u16 (rpm) | Fan speed diagnostics |
| 85 | Supply fan speed | u16 (rpm) | Fan speed diagnostics |
| 87 | Nominal ventilation value | u8 | Configured nominal airflow |
| 88 | TSP count V/H | u8 | Total number of TSP registers supported |

If the CWL supports a data ID, it will respond with Read-Ack. If not, it will either not respond or send Unknown-DataId.

### Polling Cycle

The BM remote polls the CWL in a fixed cycle of approximately 11 seconds containing:

1. Master configuration (ID 2) — Write-Data
2. Status V/H (ID 70) — Read-Data with master control flags
3. Control setpoint V/H (ID 71) — Write-Data
4. Config/member-ID V/H (ID 74) — Read-Data
5. Relative ventilation (ID 77) — Read-Data
6. Supply inlet temperature (ID 80) — Read-Data
7. Exhaust inlet temperature (ID 82) — Read-Data
8. Master product version (ID 126) — Write-Data
9. Slave product version (ID 127) — Read-Data (no response from CWL)
10. TSP index/value V/H (ID 89) — Read-Data, rotates through different index each cycle
11. Fault flags/code V/H (ID 72) — Read-Data (sometimes included)

## ESP32 Firmware Guide

### Minimal Polling Loop

To replicate the BM's communication, the ESP32 firmware should send these requests in a loop (~1 second between each):

```
1. Write-Data  ID=2   value=0x0012        (master config: member-id=18)
2. Read-Data   ID=70  value=0x0N00        (status: HI byte = control flags, N = bypass bit)
3. Write-Data  ID=71  value=0x000L        (ventilation level: L = 0-3)
4. Read-Data   ID=74  value=0x0000        (read config/member-id)
5. Read-Data   ID=77  value=0x0000        (read relative ventilation %)
6. Read-Data   ID=80  value=0x0000        (read supply inlet temp)
7. Read-Data   ID=82  value=0x0000        (read exhaust inlet temp)
8. Read-Data   ID=89  value=0xII00        (read TSP index II)
```

### Bypass Control

In step 2, set the HI byte of the request value:
- `0x0100` — bypass closed, ventilation enabled (ch=on)
- `0x0300` — bypass open, ventilation enabled (ch=on, bypass=on)

The slave response HI byte confirms the state:
- Bit 0: fault
- Bit 1: ventilation/bypass active

### Ventilation Levels

In step 3, set the LO byte:
- `0x0000` — Off (0%)
- `0x0001` — Reduced (51%)
- `0x0002` — Normal (67%)
- `0x0003` — Party (100%)

## Hardware

### Capture Setup (passive analysis)

- **Saleae Logic 2** analyzer tapping the OpenTherm bus between BM remote and CWL unit
- Analog capture at 1.5625 MHz sample rate
- OpenTherm signal: Manchester encoded at ~1000 baud
- Request (master→slave): ~10.8V peak
- Response (slave→master): ~7.3V peak

### Controller Hardware

- **DIYLess Master OpenTherm Shield** — handles OpenTherm electrical signaling
- **ESP32** (e.g. WeMos D1 mini ESP32) — runs custom firmware
- **[ihormelnyk/opentherm_library](https://github.com/ihormelnyk/opentherm_library)** — Arduino library for OpenTherm master communication

The DIYLess shield acts as the OpenTherm master, replacing the BM remote control. The ESP32 can then:
- Read all sensor values (temperatures, ventilation %, status, faults)
- Set ventilation level by writing to Data ID 71 (values 0-3)
- Control bypass via Status V/H (ID 70) master flags
- Read TSP registers for diagnostics
- Publish data via MQTT to Home Assistant

### ESPHome Compatibility

The official ESPHome OpenTherm component does **not** support V/H (ventilation/heat-recovery) data IDs 70-91. It only supports boiler/heating control. A custom firmware is required for the Wolf CWL.

## Building

```bash
cd decode
go build -o decode .
```

Requires Go 1.26+.
