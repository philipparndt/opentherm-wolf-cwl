## Context

The decoder currently outputs one format ("verbose") showing raw frame data including start/stop bits, parity, spare nibble, hex values, and multiple numeric representations (decimal, HI/LO bytes, f8.8). This is useful for protocol debugging but impractical for monitoring a ventilation system or feeding into ESPHome.

The captured signal is a Wolf CWL ventilation/heat-recovery unit communicating over OpenTherm. The data IDs seen are V/H-specific (70-91, 126-127) and include temperatures, ventilation position, status flags, and configuration.

## Goals / Non-Goals

**Goals:**
- Default output shows human-readable values: temperatures with °C, percentages with %, status flags as named booleans, paired request/response on one logical line
- Current verbose output preserved behind `--verbose` / `-v` flag
- Value interpretation is data-ID-aware: status flags get bit-level decoding, temperatures use f8.8 format, percentages show integer values
- ESPHome-friendly sensor names in output (snake_case identifiers suitable for use as ESPHome entity IDs)

**Non-Goals:**
- Generating actual ESPHome YAML config (just naming the values in a compatible way)
- Supporting all 256 OpenTherm data IDs (only the V/H subset and common IDs seen in real captures)
- Interactive or streaming output

## Decisions

### 1. Output structure: grouped request/response pairs

Pair consecutive REQ+RSP packets by matching data IDs. Display as a single logical exchange showing the response value (which is the meaningful one). Show unpaired packets individually.

Example readable output:
```
Supply inlet temperature:   17.4 °C
Exhaust inlet temperature:  22.0 °C
Relative ventilation:       51 %
Status V/H:                 fault=off ventilation=on cooling=off dhw=off
Control setpoint V/H:       1
Fault flags/code V/H:       flags=0x00 code=3
```

### 2. Value interpretation per data-ID type

Define interpretation types:
- **flag8_flag8**: Two bytes of flag bits (IDs 0, 70) — decode named bits
- **f8.8**: Signed fixed-point temperature (IDs 80-83, 24-28) — display as `XX.X °C`
- **u8**: Unsigned byte value (IDs 77, 71) — display as integer with unit (% or setpoint)
- **u16**: Unsigned 16-bit (IDs 116-123) — display as integer
- **product_version**: HI=type, LO=version (IDs 126-127)
- **config_memberid**: HI=flags, LO=member-id (IDs 2, 74)
- **tsp**: HI=index, LO=value (IDs 11, 89)

### 3. Flag `--verbose` on root and subcommands

Add a persistent flag `--verbose` / `-v` to the root cobra command. When set, call the existing `Decode()` function. When not set (default), call a new `DecodeReadable()` function.

### 4. ESPHome-friendly identifiers

Each data ID gets a snake_case identifier (e.g., `supply_inlet_temperature`, `exhaust_inlet_temperature`, `relative_ventilation`) that can serve as an ESPHome sensor entity ID.

## Risks / Trade-offs

- **[Incomplete ID coverage]** → Only V/H and common IDs are interpreted initially. Unknown IDs fall back to showing raw hex. Mitigation: easy to extend the interpretation table later.
- **[Paired output assumes REQ+RSP ordering]** → If a response is missing, the request will appear alone. Mitigation: acceptable since real captures have consistent pairing.
