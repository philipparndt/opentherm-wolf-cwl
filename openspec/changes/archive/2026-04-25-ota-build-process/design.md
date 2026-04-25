## Context

The reference project (unifi-doorbell) has a proven OTA deployment pipeline:
1. Vite builds web UI → `data/` directory
2. `gzip_webfiles.py` compresses `data/*.{html,js,css}` → `data/*.gz` (runs before LittleFS build)
3. PlatformIO builds `littlefs.bin` from `data/` (containing only `.gz` files)
4. `upload-ota.sh` authenticates via HTTP, uploads `firmware.bin` and/or `littlefs.bin` to the device
5. ESPAsyncWebServer serves `.gz` files with `Content-Encoding: gzip` headers

Our project has step 1 (Vite builds to `data/`), the OTA upload endpoint exists in the webserver, but steps 2-4 are missing.

## Goals / Non-Goals

**Goals:**
- Gzip web assets before LittleFS build (reduces flash usage ~60-70%)
- `upload-ota.sh` for command-line OTA deployment
- Filesystem OTA endpoint (`/api/ota/filesystem`) for updating the web UI without reflashing firmware
- Serve gzipped static files correctly

**Non-Goals:**
- Changing the web UI build process (Vite config is fine as-is)
- Adding a progress bar to the web UI OTA (the current simple upload works)

## Decisions

### 1. Adapt `upload-ota.sh` from reference with minimal changes

Replace doorbell-specific text and defaults. Keep the robust curl-based upload with progress tracking, reboot detection, and re-authentication between filesystem and firmware uploads. Change login endpoint to `/api/login` (our endpoint name), default host to `wolf-cwl.local`.

### 2. Copy `gzip_webfiles.py` directly

The script is generic — reads files from `data/`, gzips them, adds as PlatformIO pre-action for `littlefs.bin`. No changes needed except removing the `__UI_BUILD_ID__` replacement (we don't use that).

### 3. Serve gzipped files via ESPAsyncWebServer

ESPAsyncWebServer's `serveStatic()` already supports gzip: if a request comes for `/app.js` and `app.js.gz` exists in LittleFS, it automatically serves the gzipped version with the correct `Content-Encoding: gzip` header. No code changes needed — it works out of the box. We just need to make sure only `.gz` files end up in `data/` (the gzip script handles this).

### 4. Add `/api/ota/filesystem` endpoint

Same pattern as `/api/ota/upload` but writes to the SPIFFS/LittleFS partition instead of the app partition. Uses `Update.begin(UPDATE_SIZE_UNKNOWN, U_SPIFFS)`.
