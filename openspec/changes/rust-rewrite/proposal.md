## Why

The current ESP32 firmware (~5,500 lines of Arduino/C++) works but relies on global mutable state, manual memory management, and has no test coverage for business logic. Rust provides memory safety, strong typing, and testable module boundaries while targeting the same ESP32 hardware. The `esp-idf-svc` ecosystem is mature enough for production use.

## What Changes

- Create a new `esp32-rust/` directory with a complete Rust firmware alongside the existing C++ version.
- Use FFI bindings to call the existing OpenTherm C++ library (timing-critical code stays proven).
- Rewrite all application logic in Rust: config management, scheduler, MQTT, web server, display, encoder, network, watchdog.
- Serve the same Preact web UI from the embedded filesystem.
- Support the same build variants: WiFi, Ethernet (Olimex ESP32-POE), and simulation mode via Cargo feature flags.

## Capabilities

### New Capabilities

_None_ — this is a 1:1 functional rewrite. All existing features are preserved.

### Modified Capabilities

_None_ — no spec-level behavior changes.

## Impact

- **New directory**: `esp32-rust/` with Cargo workspace, build scripts, and all Rust source.
- **OpenTherm FFI**: C++ library compiled as a static library linked into the Rust binary.
- **Build system**: Cargo + `espflash` replaces PlatformIO. New Makefile targets for Rust builds.
- **Existing code**: `esp32/` directory unchanged — both versions can coexist and be flashed independently.
