## Context

All ~45 user-visible strings on the OLED are hardcoded in English across `display.rs` and `cwl_data.rs`. The web UI (Preact SPA in `esp32/web/src/`) also has English-only labels. There is no i18n infrastructure. The device targets German-speaking households primarily.

The OLED is 128x64 pixels using BDF fonts (helvR08 small, helvB12 medium, helvB14 large). German strings tend to be longer than English — this must be accounted for to fit the display.

## Goals / Non-Goals

**Goals:**
- Support exactly two languages: English (default) and German
- Language setting persisted in NVS and changeable from both OLED and web UI
- All OLED strings translated (headers, labels, status values, hints)
- Web UI translates all visible text based on the language setting
- Strings fit within the 128px display width (German strings verified for length)

**Non-Goals:**
- Supporting more than 2 languages (but design should not preclude it)
- Translating MQTT topic names or API JSON keys
- Translating log messages
- Right-to-left or complex script support
- Font changes (the existing Latin-1 BDF fonts cover German umlauts: ä, ö, ü, ß)

## Decisions

### 1. Translation module: static string tables indexed by enum

**Choice**: Create `src/i18n.rs` with a `Language` enum (`En`, `De`) and a `Strings` struct holding `&'static str` references for all translatable strings. Two const instances (`EN` and `DE`) provide the tables. A function `tr(lang: Language) -> &'static Strings` returns the active table.

**Rationale**: Zero-cost at runtime (just pointer lookups), no heap allocation, compile-time guarantees that all strings exist in both languages. Fits the embedded no-alloc-preferred pattern.

**Alternative considered**: HashMap/BTreeMap of string keys → rejected for memory overhead on ESP32. Macro-based approach → added complexity for only 2 languages.

### 2. Language stored in AppConfig, accessible via shared AppState

**Choice**: Add `pub language: Language` to `AppConfig` (default: `Language::En`). Serialize to NVS as `u8` (0=En, 1=De). Display reads it from the locked state on each render frame.

**Rationale**: Config is already persisted to NVS. The display already locks AppState to read `cwl_data` — one extra field read per frame has negligible cost.

### 3. New OLED page "Settings" for language selection

**Choice**: Add a `Settings` page after `System` (PAGE_COUNT becomes 7). In edit mode, the rotary encoder toggles between "English" and "Deutsch". The page shows the current language when not in edit mode.

**Rationale**: Keeps language accessible without the web UI. A dedicated settings page can host future settings too. One extra page in the dot carousel is acceptable.

### 4. German string fitting strategy

**Choice**: For strings that would exceed display width in German, use abbreviations or shorter synonyms. For the large font (helvB14), max ~8-9 chars fit. Headers (helvR08) fit ~20 chars.

Key translations fitting constraints:
- "Ventilation" → "Lüftung" (7 chars, fits large)
- "Set Level" → "Stufe" (5 chars)
- "Off Duration" → "Aus-Dauer" (9 chars)
- "Summer Mode" → "Sommermodus" (11 chars, fits header)
- "Reduced" → "Reduziert" (9 chars, fits large)
- "Normal" → "Normal" (same)
- "Party" → "Party" (same)
- "Schedule" → "Zeitplan" (8 chars, fits large)
- "Intake" → "Zuluft" (6 chars)
- "Outlet" → "Abluft" (6 chars)
- "Status" → "Status" (same)
- "System" → "System" (same)
- "Settings" → "Einstellungen" (13 chars, header only)

### 5. Web UI i18n approach

**Choice**: Create a `translations.ts` file with EN/DE objects keyed by string ID. The App component reads the language from the config API response and passes it as context. Components use a `t(key)` helper.

**Rationale**: Simple, no external i18n library needed for ~30 web strings. The language is fetched with config on app load, so no extra API call.

### 6. REST API changes

**Choice**: Add `"language": "en"|"de"` to the config GET/PUT JSON endpoints. This reuses the existing config save/load infrastructure.

## Risks / Trade-offs

- **[German strings may not fit]** → Mitigated by pre-verifying pixel widths for each string against font metrics; use abbreviations where needed
- **[Umlauts may not render]** → The helvR08/helvB12/helvB14 fonts include Latin-1 supplement (ä=0xE4, ö=0xF6, ü=0xFC, ß=0xDF) — verified in U8g2 font data
- **[Extra flash usage]** → ~2KB for string tables, negligible on ESP32 with 4MB flash
- **[7 pages in carousel]** → Slightly more scrolling; acceptable since settings page is rarely needed
