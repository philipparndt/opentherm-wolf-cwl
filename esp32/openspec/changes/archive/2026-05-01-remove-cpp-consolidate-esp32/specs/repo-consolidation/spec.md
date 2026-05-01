## ADDED Requirements

### Requirement: Single esp32 project directory
The repository SHALL contain a single `esp32/` directory containing the Rust firmware and the web UI source. No C++ PlatformIO project SHALL exist.

#### Scenario: Directory structure after consolidation
- **WHEN** the repository is checked out
- **THEN** `esp32/` SHALL contain the Rust firmware (Cargo.toml, src/, Makefile) and `esp32/web/` SHALL contain the Preact web UI source

### Requirement: Web build paths are project-local
The Makefile SHALL reference the web source and output using project-local paths (`web/` and `data/`) without cross-directory references.

#### Scenario: Building web UI
- **WHEN** `make build-web` is run from the `esp32/` directory
- **THEN** the web UI SHALL build from `web/` and output to `data/`
