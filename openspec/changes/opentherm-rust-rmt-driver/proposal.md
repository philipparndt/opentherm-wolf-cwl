## Why

The current OpenTherm driver is a C++ component (`components/opentherm/`) bridged to Rust via an FFI layer. It disables interrupts for the entire 34-bit frame transmission — at 500 µs per half-bit that is ~34 ms of global interrupt disable per request. During OTA firmware flashing the ESP32 WiFi/TCP stack relies on frequent interrupt service; the 34 ms blackout causes packet drops, retransmits, and perceived UI lag. Additionally, the C++ component adds build complexity: a hand-rolled cross-compiler invocation in `build.rs`, an Arduino.h shim, and ~30 ESP-IDF include paths wired up manually.

## What Changes

- New pure-Rust OpenTherm driver in `src/opentherm.rs` using the ESP32 RMT peripheral for both TX and RX. RMT handles precise µs-level timing in hardware; no interrupt disabling is required.
- `src/opentherm_ffi.rs` deleted. `ot_master.rs` updated to import from `src/opentherm.rs`; the public API is kept identical so no logic changes are needed.
- `components/opentherm/` directory deleted (C++ source, Arduino.h shim, FFI wrappers).
- `build.rs` simplified: `compile_opentherm()` and all its cross-compiler discovery logic removed. `cc` build-dependency removed from `Cargo.toml`.
- `simulate-ot` feature retained and still works via `#[cfg(feature = "simulate-ot")]` gating inside the new module.

## Capabilities

### New Capabilities

_None_ — protocol behaviour is identical to the current driver.

### Modified Capabilities

_None_ — the same OpenTherm master role, same data IDs, same 1 s timeout, same 100 ms recovery delay.

## Impact

- **`src/opentherm.rs`** (new): RMT-based driver — frame encode/decode, TX, RX, parity, status.
- **`src/opentherm_ffi.rs`** (deleted): replaced by the new module.
- **`src/ot_master.rs`**: one-line import change (`opentherm_ffi` → `opentherm`).
- **`src/main.rs`**: same one-line import change if it re-exports the type.
- **`components/opentherm/`** (deleted): entire C++ component tree.
- **`build.rs`**: `compile_opentherm()` function and its dependencies removed; WiFi credential forwarding untouched.
- **`Cargo.toml`**: `cc = "1"` build-dependency removed.
