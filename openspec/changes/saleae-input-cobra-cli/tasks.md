## 1. Project restructure and cobra setup

- [ ] 1.1 Add `github.com/spf13/cobra` dependency to `go.mod`
- [ ] 1.2 Extract shared types (`Sample`, `Edge`, `Packet`) into `decoder/types.go`
- [ ] 1.3 Extract decoder logic (`findPackets`, `findEdges`, `analyzeBitTiming`, `decodeManchester`, `avg`, OpenTherm maps, frame decoding/output) into `decoder/opentherm.go`
- [ ] 1.4 Extract CSV reader (`loadCSV`) into `reader/csv.go`

## 2. SAL reader

- [ ] 2.1 Implement `reader/sal.go`: open `.sal` ZIP, extract `meta.json`, parse sample rate from `data.legacySettings.sampleRate.analog`
- [ ] 2.2 Implement `analog-0.bin` parsing: read packed `float32` little-endian values, compute timestamps as `index / sampleRate`, return `[]Sample`
- [ ] 2.3 Add error handling for missing `analog-0.bin`, invalid ZIP, and missing sample rate in `meta.json`

## 3. Cobra CLI commands

- [ ] 3.1 Create `cmd/root.go` with root command that auto-detects format by file extension and runs the appropriate reader + decoder
- [ ] 3.2 Create `cmd/csv.go` with `csv` subcommand that uses the CSV reader
- [ ] 3.3 Create `cmd/sal.go` with `sal` subcommand that uses the SAL reader
- [ ] 3.4 Update `main.go` to just call `cmd.Execute()`

## 4. Verification

- [ ] 4.1 Build and test with existing CSV file (`analog.csv`)
- [ ] 4.2 Build and test with `.sal` file (`Session 0.sal`)
- [ ] 4.3 Verify auto-detection works for both formats
- [ ] 4.4 Verify help text displays when no arguments provided
