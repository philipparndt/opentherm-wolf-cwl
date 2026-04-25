## 1. Build Scripts

- [x] 1.1 Create `scripts/gzip_webfiles.py` — gzip `data/*.{html,js,css}` before LittleFS build
- [x] 1.2 Update `platformio.ini` — add `gzip_webfiles.py` to `extra_scripts`
- [x] 1.3 Create `upload-ota.sh` — OTA upload script adapted from reference project

## 2. Webserver Updates

- [x] 2.1 Add `/api/ota/filesystem` endpoint — LittleFS OTA upload via `U_SPIFFS`
- [x] 2.2 Add `/api/auth/login` alias — so OTA script's login path works alongside existing `/api/login`

## 3. Verification

- [x] 3.1 Build verification — firmware SUCCESS, gzip working (JS 65.5%, CSS 59.6%, HTML 25.1% reduction)
