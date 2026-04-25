## ADDED Requirements

### Requirement: Verbose flag
The CLI SHALL accept a `--verbose` / `-v` flag. When set, the system SHALL use the existing verbose output format. When not set, the system SHALL use the new readable output format.

#### Scenario: Default output is readable
- **WHEN** user runs `decode <file>` without `--verbose`
- **THEN** the system SHALL display the readable output format

#### Scenario: Verbose flag enables detailed output
- **WHEN** user runs `decode <file> --verbose`
- **THEN** the system SHALL display the existing verbose output with raw hex, parity, start/stop bits, and all numeric representations

#### Scenario: Verbose flag works with subcommands
- **WHEN** user runs `decode csv <file> -v` or `decode sal <file> --verbose`
- **THEN** the system SHALL display verbose output for those subcommands
