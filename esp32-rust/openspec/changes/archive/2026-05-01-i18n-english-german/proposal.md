## Why

The UI (both OLED display and web interface) is currently English-only with all strings hardcoded. The device is used in German-speaking households where German labels would be more natural and accessible. Adding bilingual support (English/German) makes the UI usable for a wider audience while keeping the implementation simple.

## What Changes

- Add a language setting (English/German) persisted in NVS config
- Create a translation module with all OLED display strings in both languages
- Update all display rendering to use translated strings instead of hardcoded English
- Add a "Language" page to the OLED display where the user can switch between EN/DE via the rotary encoder
- Add a language selector to the web UI settings
- Expose the language setting via the REST API (GET/PUT config)
- Translate web UI strings to German (client-side, driven by the language setting from the API)

## Capabilities

### New Capabilities
- `oled-i18n`: Translation system for OLED display strings with English and German support, including a language selection page on the display
- `web-i18n`: Language selector in the web UI and translated labels/strings for German

### Modified Capabilities

## Impact

- `esp32-rust/src/display.rs` — All draw functions use translation lookups; new Language page added
- `esp32-rust/src/config.rs` — New `language` field in AppConfig (persisted to NVS)
- `esp32-rust/src/webserver.rs` — Language field in config GET/PUT endpoints
- `esp32/web/src/App.tsx` — Language selector in settings, translated UI strings
- New file: `esp32-rust/src/i18n.rs` — Translation module with string tables
