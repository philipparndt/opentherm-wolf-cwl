## ADDED Requirements

### Requirement: Cobra CLI with subcommands
The app SHALL use cobra to provide a CLI with the following command structure:
- `decode csv <file>` — decode from Saleae CSV export
- `decode sal <file>` — decode from Saleae `.sal` capture file
- `decode <file>` — auto-detect format by file extension

#### Scenario: Decode CSV via subcommand
- **WHEN** user runs `decode csv analog.csv`
- **THEN** the system SHALL load the CSV file and decode OpenTherm frames, producing the same output as the current tool

#### Scenario: Decode SAL via subcommand
- **WHEN** user runs `decode sal session.sal`
- **THEN** the system SHALL load the `.sal` file and decode OpenTherm frames

#### Scenario: Auto-detect CSV
- **WHEN** user runs `decode analog.csv` (root command, no subcommand)
- **THEN** the system SHALL detect `.csv` extension and use the CSV reader

#### Scenario: Auto-detect SAL
- **WHEN** user runs `decode session.sal` (root command, no subcommand)
- **THEN** the system SHALL detect `.sal` extension and use the SAL reader

#### Scenario: Unknown file extension
- **WHEN** user runs `decode file.xyz` with an unrecognized extension
- **THEN** the system SHALL exit with an error message listing supported formats (`.csv`, `.sal`)

### Requirement: No arguments shows usage
The app SHALL display usage/help text when invoked with no arguments.

#### Scenario: No arguments
- **WHEN** user runs `decode` with no arguments
- **THEN** the system SHALL display cobra-generated help text showing available commands and usage
