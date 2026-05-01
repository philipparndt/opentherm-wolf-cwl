## ADDED Requirements

### Requirement: Translation module provides string tables for English and German
The system SHALL provide a translation module (`i18n.rs`) with a `Language` enum (`En`, `De`) and static string tables for both languages covering all user-visible OLED strings.

#### Scenario: Accessing English strings
- **WHEN** the language is set to `En`
- **THEN** all display strings SHALL be in English (e.g. "Ventilation", "Reduced", "Off", "Summer", "Winter")

#### Scenario: Accessing German strings
- **WHEN** the language is set to `De`
- **THEN** all display strings SHALL be in German (e.g. "Lüftung", "Reduziert", "Aus", "Sommer", "Winter")

### Requirement: Language setting persisted in config
The `AppConfig` SHALL include a `language` field of type `Language` that is persisted to NVS. The default SHALL be `Language::En`.

#### Scenario: Language persists across reboots
- **WHEN** the user sets the language to German and the device reboots
- **THEN** the language SHALL remain German after reboot

#### Scenario: Default language on fresh device
- **WHEN** no language has been configured (fresh NVS)
- **THEN** the language SHALL default to English

### Requirement: All OLED display strings use the translation module
Every user-visible string rendered on the OLED display SHALL be sourced from the translation module based on the active language setting. No user-visible strings SHALL be hardcoded in the display rendering functions.

#### Scenario: Home page in German
- **WHEN** language is German and the home page is displayed
- **THEN** the header SHALL show "Lüftung" and the level name SHALL show "Reduziert"/"Normal"/"Party"/"Aus" as appropriate

#### Scenario: Edit mode hints in German
- **WHEN** language is German and edit mode is active
- **THEN** hints SHALL be in German (e.g. "drehen: wählen  drücken: ok")

#### Scenario: Status page in German
- **WHEN** language is German and the status page is displayed
- **THEN** labels SHALL be in German (e.g. "Störung:", "Filter:", "Modus:", "Luftstrom:")

### Requirement: Settings page for language selection on OLED
The OLED SHALL have a "Settings" page accessible via the rotary encoder page navigation. In edit mode, the user can toggle between "English" and "Deutsch". Confirming applies the change and persists it.

#### Scenario: Navigating to settings page
- **WHEN** the user scrolls past the System page
- **THEN** the Settings page SHALL be displayed showing the current language

#### Scenario: Changing language via OLED
- **WHEN** the user enters edit mode on the Settings page and selects "Deutsch"
- **THEN** the language SHALL change to German immediately and all subsequent pages SHALL display in German

### Requirement: German strings fit within display constraints
All German translation strings SHALL fit within the available pixel width for their respective font and display area (128px width). Headers (small font) SHALL NOT exceed 125px. Level names and centered text (large font) SHALL NOT exceed 124px.

#### Scenario: Long German header
- **WHEN** the German header text is rendered (e.g. "Einstellungen")
- **THEN** it SHALL fit within 125 pixels at helvR08 font size

#### Scenario: German level names
- **WHEN** German level names are rendered in large font (e.g. "Reduziert", "Zeitplan")
- **THEN** they SHALL fit within 124 pixels at helvB14 font size
