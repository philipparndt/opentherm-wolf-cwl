## 1. Targeted NVS Helper

- [x] 1.1 Add `saveBypassState()` function in `config_manager.cpp` that opens NVS and writes only the `bypass_open` key
- [x] 1.2 Declare `saveBypassState()` in `config_manager.h`

## 2. Display Mode Switch

- [x] 2.1 In `exitEditMode()` ventilation path: remove `saveConfig()` call (level is kept in memory only, override is persisted by `setVentilationManualOverride()`)
- [x] 2.2 In `exitEditMode()` bypass path: replace `saveConfig()` with `saveBypassState()`

## 3. MQTT Mode Switch

- [x] 3.1 In `mqttCallback()` ventilation level handler: remove `saveConfig()` call
- [x] 3.2 In `mqttCallback()` bypass handler: replace `saveConfig()` with `saveBypassState()`

## 4. Verification

- [ ] 4.1 Build firmware and verify no compilation errors
