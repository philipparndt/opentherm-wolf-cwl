## Context

The `decode` Go app is a single-file tool (`main.go`, ~440 lines) that reads a Saleae Logic 2 CSV export and decodes OpenTherm Manchester-encoded frames. It currently takes a single positional argument (the CSV path) with no CLI structure. Users must manually export CSV from Saleae Logic 2 before decoding.

Saleae `.sal` files are ZIP archives containing:
- `analog-0.bin`: raw `float32` voltage samples, one per sample tick
- `meta.json`: capture metadata including sample rate (`legacySettings.sampleRate.analog`), channel config, and capture timestamps
- `trigger-store.bin`: trigger data (not needed for decoding)

The existing CSV format has headers `Time [s],Channel 0` with time in seconds and voltage as float.

## Goals / Non-Goals

**Goals:**
- Read `.sal` files directly without manual CSV export
- Continue supporting Saleae CSV exports as input
- Provide clean CLI with cobra subcommands
- Auto-detect format by file extension

**Non-Goals:**
- Supporting digital channels from `.sal` files
- Supporting multi-channel analog captures (only channel 0)
- Real-time capture or Saleae device communication
- Output format changes (keep existing stdout text output)

## Decisions

### 1. Project structure: cmd/ package pattern

Split `main.go` into:
- `main.go` — cobra root command setup only
- `cmd/root.go` — root `decode <file>` command with auto-detect
- `cmd/csv.go` — `decode csv <file>` subcommand
- `cmd/sal.go` — `decode sal <file>` subcommand
- `reader/csv.go` — CSV sample loader (extracted from current `loadCSV`)
- `reader/sal.go` — `.sal` ZIP/binary reader
- `decoder/opentherm.go` — packet finding, edge detection, Manchester decoding, OT frame parsing (extracted from current main.go)

**Rationale**: Keeps concerns separated. The `reader` package provides a common `[]Sample` output regardless of input format. The `decoder` package is input-format agnostic.

### 2. SAL binary format parsing

The `analog-0.bin` file contains raw `float32` (IEEE 754, little-endian) samples at a fixed sample rate. Time values are reconstructed as `sampleIndex / sampleRate`. The sample rate comes from `meta.json` at `data.legacySettings.sampleRate.analog`.

**Rationale**: Avoids depending on any Saleae SDK. The format is straightforward — just packed floats.

### 3. Auto-detection strategy

The root command checks file extension: `.sal` → sal reader, `.csv` → csv reader. No magic-byte sniffing needed since extensions are unambiguous.

### 4. Shared Sample type

Define `Sample` struct in a shared package (e.g., `model/` or keep in `decoder/`) so both readers produce the same type.

## Risks / Trade-offs

- **[SAL format undocumented]** → Saleae Logic 2's `.sal` format is not officially documented. Parsing is based on observed structure (ZIP with `analog-0.bin` as packed float32). If Saleae changes the format, the reader may break. Mitigation: validate `meta.json` version field and fail clearly on unsupported versions.
- **[Large files]** → A 50MB `analog-0.bin` at 4 bytes/sample = ~12.5M samples. All loaded into memory. Mitigation: acceptable for this use case; streaming would add complexity without clear benefit.
- **[Single channel assumption]** → Only reads `analog-0.bin` (channel 0). Multi-channel captures would need `analog-1.bin`, etc. Mitigation: error clearly if expected file is missing.
