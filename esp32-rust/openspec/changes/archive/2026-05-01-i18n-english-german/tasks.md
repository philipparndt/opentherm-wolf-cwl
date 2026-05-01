## 1. Create translation module

- [x] 1.1 Create `src/i18n.rs` with `Language` enum (`En`, `De`), `Strings` struct with all translatable fields, and const `EN`/`DE` string tables
- [x] 1.2 Add `mod i18n;` to `main.rs` and make `Language` and `tr()` accessible

## 2. Add language to config

- [x] 2.1 Add `pub language: Language` field to `AppConfig` in `config.rs` (default: `Language::En`)
- [x] 2.2 Update NVS serialization/deserialization to persist language as `u8` (0=En, 1=De)

## 3. Add Settings page to OLED

- [x] 3.1 Add `Settings` variant to `Page` enum, update `PAGE_COUNT` to 7, update `from_index`
- [x] 3.2 Implement `draw_settings()` function showing current language, with edit mode to toggle between "English"/"Deutsch"
- [x] 3.3 Update `enter_edit_mode()` and `exit_edit_mode()` to handle the Settings page (toggle language, persist config)

## 4. Replace hardcoded strings in display

- [x] 4.1 Update `draw_home()` to use `tr(lang)` for header, level names, hints, and status labels
- [x] 4.2 Update `draw_bypass()` to use translated strings
- [x] 4.3 Update `draw_temp_in()` and `draw_temp_out()` to use translated labels
- [x] 4.4 Update `draw_status()` to use translated labels
- [x] 4.5 Update `draw_system()` to use translated labels
- [x] 4.6 Update boot screen and overlay strings to use translations

## 5. Update REST API

- [x] 5.1 Add `"language"` field to config GET response in `webserver.rs`
- [x] 5.2 Parse `"language"` field from config PUT request and update AppConfig

## 6. Web UI translations

- [x] 6.1 Create `esp32/web/src/translations.ts` with EN/DE string objects
- [x] 6.2 Add language selector to the settings section in `App.tsx`
- [x] 6.3 Replace hardcoded English strings in `App.tsx` with `t(key)` calls using active language
- [x] 6.4 Update `StatusTab.tsx` to use translated strings

## 7. Verify

- [x] 7.1 Run `cargo check` to confirm Rust code compiles
- [x] 7.2 Run `cd esp32/web && npm run build` to confirm web UI builds
