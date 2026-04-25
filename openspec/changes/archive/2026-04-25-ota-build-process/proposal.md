## Why

The firmware has OTA upload support in the web server (`/api/ota/upload`) and a Makefile with `deploy` targets, but the actual OTA upload script (`upload-ota.sh`) doesn't exist yet. Also missing: the `gzip_webfiles.py` PlatformIO script that compresses web assets before building the LittleFS image — without it, the ESP32 serves uncompressed files, wasting flash and bandwidth. The reference project (unifi-doorbell) has both of these working. Additionally, the web server needs to serve gzipped files and support the filesystem OTA endpoint.

## What Changes

- Add `upload-ota.sh` — shell script for OTA firmware and filesystem uploads via HTTP (adapted from reference project)
- Add `scripts/gzip_webfiles.py` — PlatformIO pre-action to gzip web assets in `data/` before LittleFS build
- Update `platformio.ini` to include the gzip script in `extra_scripts`
- Add `/api/ota/filesystem` endpoint in `webserver.cpp` for LittleFS OTA
- Configure ESPAsyncWebServer to serve `.gz` files with proper `Content-Encoding` headers
- Align login endpoint: add `/api/auth/login` alias so the OTA script works

## Capabilities

### New Capabilities

(none — these are build/deploy tooling improvements, not new firmware features)

### Modified Capabilities

(none)

## Impact

- **New files**: `upload-ota.sh`, `scripts/gzip_webfiles.py`
- **Modified files**: `platformio.ini` (extra_scripts), `webserver.cpp` (gzip serving, filesystem OTA, login alias)
- No new dependencies
