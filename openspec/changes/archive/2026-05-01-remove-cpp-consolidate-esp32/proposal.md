## Why

The C++ ESP32 application (`esp32/`) is legacy — all functionality has been reimplemented in the Rust firmware (`esp32-rust/`). The web UI source (`esp32/web/`) is already used exclusively by the Rust build. Keeping the old C++ project causes confusion about which is the active codebase. Consolidating to a single `esp32/` directory simplifies the repo structure.

## What Changes

- Move `esp32/web/` into `esp32-rust/web/` (co-locate web source with the Rust project)
- Delete the entire `esp32/` directory (C++ source, PlatformIO config, scripts, data output)
- Rename `esp32-rust/` to `esp32/`
- Update the Makefile paths (`WEB_SRC`, `WEB_OUT`) to reference `./web` and `./data` (now local)
- Update any other cross-references (gitignore, README, etc.)

## Capabilities

### New Capabilities

### Modified Capabilities

## Impact

- **BREAKING**: The `esp32/` path changes meaning (was C++, becomes Rust)
- `esp32-rust/Makefile` — path updates for web build
- Root `.gitignore` — may reference `esp32/data/` or `esp32-rust/` paths
- `README.md` — references to project structure
- `openspec/` at root — references to `esp32-rust/` if any
- Git history preserved via `git mv`
