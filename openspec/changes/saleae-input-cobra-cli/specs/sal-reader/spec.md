## ADDED Requirements

### Requirement: Read Saleae .sal capture files
The system SHALL accept `.sal` files as input and extract analog voltage samples from them. A `.sal` file is a ZIP archive containing `analog-0.bin` (packed `float32` little-endian samples) and `meta.json` (capture metadata).

#### Scenario: Decode from .sal file
- **WHEN** user provides a valid `.sal` file path
- **THEN** the system SHALL extract `analog-0.bin` and `meta.json` from the ZIP, parse the binary float32 samples, reconstruct timestamps using the sample rate from `meta.json`, and produce voltage samples identical in structure to CSV-loaded samples

#### Scenario: Missing analog data in .sal
- **WHEN** user provides a `.sal` file that does not contain `analog-0.bin`
- **THEN** the system SHALL exit with an error message indicating no analog data was found

#### Scenario: Invalid or corrupt .sal file
- **WHEN** user provides a file with `.sal` extension that is not a valid ZIP archive
- **THEN** the system SHALL exit with an error message indicating the file could not be read

### Requirement: Extract sample rate from meta.json
The system SHALL read the analog sample rate from `meta.json` at the path `data.legacySettings.sampleRate.analog` and use it to compute sample timestamps as `index / sampleRate`.

#### Scenario: Sample rate extraction
- **WHEN** a `.sal` file is opened
- **THEN** the system SHALL read the sample rate from `meta.json` and use it to generate accurate time values for each sample

#### Scenario: Missing sample rate in meta.json
- **WHEN** `meta.json` does not contain a valid sample rate at the expected path
- **THEN** the system SHALL exit with an error message indicating the sample rate could not be determined
