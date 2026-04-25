## Why

The OpenTherm decoder currently only reads Saleae Logic CSV exports, requiring a manual export step from Saleae Logic 2. Supporting native `.sal` capture files directly eliminates this step and preserves the full capture metadata. Additionally, the app has no CLI structure — adding cobra provides a clean command hierarchy for current and future input formats.

## What Changes

- Restructure the Go app to use cobra CLI with subcommands (`decode csv`, `decode sal`)
- Add `.sal` file reader that extracts `analog-0.bin` and `meta.json` from the ZIP archive, parses the binary float samples and sample rate, and produces the same `[]Sample` slice the decoder already uses
- Support Saleae CSV exports (current `Time [s],Channel 0` format) via the existing CSV loader, wired as `decode csv`
- Auto-detect input format by file extension when a bare `decode <file>` is used

## Capabilities

### New Capabilities
- `sal-reader`: Read Saleae Logic 2 `.sal` capture files (ZIP containing `analog-0.bin` raw float data + `meta.json` with sample rate/channel config) and convert to voltage samples
- `cobra-cli`: Restructure the app with cobra subcommands (`decode csv <file>`, `decode sal <file>`, and root `decode <file>` with auto-detection)

### Modified Capabilities

## Impact

- `decode/main.go` will be restructured into multiple files (cmd package, readers, decoder logic)
- New dependency: `github.com/spf13/cobra`
- `go.mod` updated with cobra dependency
- The `analog-0.bin` binary format in `.sal` files uses raw `float32` samples at the sample rate specified in `meta.json` (`legacySettings.sampleRate.analog`)
