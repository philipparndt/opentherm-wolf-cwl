## 1. Value interpretation

- [x] 1.1 Add data-ID interpretation table in `decoder/` mapping each ID to: display name, snake_case ID, value type (flag8, f8.8, u8, u16, product_version, config_memberid, tsp), and unit string
- [x] 1.2 Implement value formatting functions for each type: `formatF88` (°C), `formatU8` (%), `formatStatusVH` (named flag bits), `formatFaultVH` (flags + code), `formatProductVersion`, `formatConfigMemberID`, `formatTSP`

## 2. Readable output mode

- [x] 2.1 Implement request/response pairing: match consecutive REQ+RSP packets by data ID, collect paired exchanges
- [x] 2.2 Implement `DecodeReadable(samples []Sample)` function that prints summary header + paired exchanges with human-readable values
- [x] 2.3 For each exchange, display: sensor name, formatted value with unit; for unknown IDs fall back to `value=0xNNNN`

## 3. CLI flag

- [x] 3.1 Add persistent `--verbose` / `-v` bool flag to root cobra command
- [x] 3.2 Update `cmd/root.go`, `cmd/csv.go`, `cmd/sal.go` to pass verbose flag and call either `Decode()` or `DecodeReadable()` accordingly

## 4. Verification

- [x] 4.1 Build and test readable output with CSV file
- [x] 4.2 Build and test readable output with SAL file
- [x] 4.3 Verify `--verbose` flag produces original verbose output
- [x] 4.4 Verify default (no flag) produces readable output
