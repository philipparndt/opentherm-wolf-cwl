## Why

The current decoder output is a verbose technical dump showing raw hex values, bit fields, and f8.8 encodings. To use the decoded data for ESPHome integration or quick monitoring, users need a clean, human-readable summary showing temperatures in °C, humidity in %, ventilation position, status flags (on/off), and fault codes — without needing to manually interpret OpenTherm data formats.

## What Changes

- Add a new default output mode ("readable") that shows decoded OpenTherm values with units, human-readable status flag descriptions, and grouped request/response pairs
- Keep the current output as a "verbose" mode selectable via `--verbose` / `-v` flag
- Interpret OpenTherm V/H (ventilation/heat-recovery) data IDs with proper value decoding:
  - Status flags (ID 70): individual bit meanings (fault, ventilation mode, cooling, etc.)
  - Temperatures (IDs 80-83): f8.8 format displayed as `XX.X °C`
  - Relative ventilation (ID 77): percentage `XX %`
  - Fault flags/code (ID 72): flag bits + fault code
  - Control setpoint V/H (ID 71): nominal ventilation setpoint
  - Config/member-ID (ID 74): member ID and config flags
  - TSP index/value (ID 89): transparent slave parameter readout
  - Product versions (IDs 126-127): product type and version

## Capabilities

### New Capabilities
- `readable-output`: Human-readable output mode with value interpretation, units, flag descriptions, and ESPHome-friendly naming
- `output-mode-flag`: CLI flag `--verbose` / `-v` to switch between readable (default) and verbose output

### Modified Capabilities

## Impact

- `decoder/opentherm.go`: add value interpretation functions and new output formatting
- `cmd/root.go`, `cmd/csv.go`, `cmd/sal.go`: add `--verbose` flag
- No new dependencies
