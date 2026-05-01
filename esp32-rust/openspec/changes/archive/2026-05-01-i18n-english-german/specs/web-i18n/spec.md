## ADDED Requirements

### Requirement: Language exposed in REST API config
The config GET endpoint SHALL include a `"language"` field with value `"en"` or `"de"`. The config PUT endpoint SHALL accept `"language"` to change the setting.

#### Scenario: Reading language from config API
- **WHEN** a GET request is made to the config endpoint
- **THEN** the response JSON SHALL include `"language": "en"` or `"language": "de"`

#### Scenario: Setting language via config API
- **WHEN** a PUT request sets `"language": "de"`
- **THEN** the language SHALL be changed to German and persisted

### Requirement: Web UI language selector
The web UI settings section SHALL include a language dropdown/toggle allowing the user to select between English and German.

#### Scenario: Changing language in web UI
- **WHEN** the user selects "Deutsch" in the language selector and saves
- **THEN** the config API SHALL be called with `"language": "de"` and the web UI SHALL re-render in German

### Requirement: Web UI displays translated strings
The web UI SHALL display all user-visible labels, headings, and messages in the selected language.

#### Scenario: Web UI in German
- **WHEN** the language is set to German
- **THEN** tab labels, button text, headings, and status labels SHALL display in German (e.g. "Lüftung", "Einstellungen", "Zeitplan", "Speichern")

#### Scenario: Web UI in English
- **WHEN** the language is set to English
- **THEN** all labels SHALL display in English (existing behavior)

### Requirement: Level names in web UI match OLED
The ventilation level names shown in the web UI SHALL match the OLED display names in the selected language.

#### Scenario: German level names consistency
- **WHEN** language is German
- **THEN** the web UI SHALL show "Aus", "Reduziert", "Normal", "Party" matching the OLED display
