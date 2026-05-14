# Wolf CWL — ESP32 Firmware

Rust firmware for the Olimex ESP32-POE board that drives a Wolf CWL ventilation unit over OpenTherm. Networking is selectable at build time (Ethernet on the Olimex POE port, or WiFi).

## Prerequisites

One-time, outside this Makefile:

1. **Rust + the Xtensa ESP toolchain via [espup](https://github.com/esp-rs/espup).**
   ```bash
   cargo install espup
   espup install
   ```
   `espup install` writes `~/export-esp.sh`, which sets `LIBCLANG_PATH` and prepends the Xtensa cross-compiler to `PATH`.

2. **Source the espup env from your shell rc** so `make` picks it up:
   ```bash
   echo 'source ~/export-esp.sh' >> ~/.zshrc && exec zsh
   ```

3. **Plug in the board** and confirm the OS sees it:
   ```bash
   ls /dev/cu.*
   ```
   Look for `cu.usbserial-*` (CP210x — native on macOS 11+, no extra driver) or `cu.wchusbserial-*` (CH340/CH341 — older boards may need the [WCH driver](https://www.wch.cn/downloads/CH341SER_MAC_ZIP.html)).

## First-time setup

```bash
cd esp32
make setup
cp .env.example .env
$EDITOR .env          # WIFI_SSID, WIFI_PASSWORD, optional PORT
```

`make setup` installs the two Rust binaries the build needs (`ldproxy`, `espflash`) and patches a Python venv quirk that otherwise breaks the ESP-IDF dependency check (see [Troubleshooting](#troubleshooting)).

## Build targets

| Target           | Network    | OpenTherm   |
|------------------|------------|-------------|
| `make dev`       | Ethernet   | Simulated   |
| `make prod`      | Ethernet   | Real        |
| `make dev-wifi`  | WiFi       | Simulated   |
| `make prod-wifi` | WiFi       | Real        |

All four build, flash, and open the serial monitor. WiFi targets require `WIFI_SSID` / `WIFI_PASSWORD` in `esp32/.env` — those credentials are forwarded by `build.rs` and compiled into the firmware as a fallback when NVS has no stored credentials (so a freshly-flashed board joins the network without going through the web setup wizard first).

`make help` lists every target.

## `.env`

Loaded by the Makefile as Makefile syntax (not shell), so:

- No quotes around values.
- Avoid `#` and `$` in passwords — both are Make-special. If you must use them, change the password.
- The file is git-ignored.

| Variable        | Used by                | Notes                                                                 |
|-----------------|------------------------|-----------------------------------------------------------------------|
| `WIFI_SSID`     | `build.rs` (env!)      | Required for `dev-wifi` / `prod-wifi`.                                |
| `WIFI_PASSWORD` | `build.rs` (env!)      | Required for `dev-wifi` / `prod-wifi`.                                |
| `PORT`          | `espflash` flag        | Optional. Defaults to `/dev/cu.wchusbserial8310`. Override per board. |

## Feature flags

Defined in `Cargo.toml`. The Makefile targets compose these; you rarely need to set them by hand:

- `wifi` / `ethernet` — pick one network stack.
- `simulate-ot` — fake OpenTherm slave for desk testing.
- `ot-uext` / `ot-ext` — which GPIO pair to use for OpenTherm (UEXT default).
- `display-rotate` — rotate the OLED 180° for upside-down mounting.
- `display-sh1106` / (default `ssd1306`) — pick OLED driver chip.

## Troubleshooting

### `Error while connecting to device`

`espflash` can't talk to whatever's at `PORT`. Confirm the port name (`ls /dev/cu.*`) and set `PORT=...` in `.env`, or pass `make ... PORT=/dev/cu.foo`. If the port is correct, hold the BOOT button while espflash resets the chip.

### `Failed to open file: partitions.csv`

A 4MB layout with NVS + factory app + a 960KB SPIFFS partition labeled `spiffs` is committed at `esp32/partitions.csv`. The SPIFFS offset/size must stay in sync with `SPIFFS_OFFSET` / `SPIFFS_SIZE` in the Makefile and the `partition_label` passed to `esp_vfs_spiffs_register` in `main.rs`. Don't edit it without updating all three.

### `Failed to run Python dependency check ... Error: 255` during `esp-idf-sys` build

The ESP-IDF Python venv ships with a `ruamel.yaml` whose `.dist-info` directory uses underscores (`ruamel_yaml-*.dist-info`), but Python 3.9's `importlib.metadata.version('ruamel.yaml')` doesn't normalize dotted names → the dep check exits 255 → CMake aborts. `make fix-python-env` symlinks the dotted names; it runs automatically before every `build` target and is also re-runnable on its own. If you see this error after a fresh `.embuild`, run:

```bash
make fix-python-env
```

### `linker 'ldproxy' not found` or `espflash: command not found`

You haven't run `make setup` yet (or `~/.cargo/bin` isn't on `PATH`). Both binaries land in `~/.cargo/bin`, which espup adds automatically — but `make` runs in `/bin/sh` and only sees the inherited `PATH`. Make sure your shell rc sources `~/export-esp.sh` and exports `~/.cargo/bin`.

### Switching between `ethernet` and `wifi` triggers a long rebuild

Expected. Cargo rebuilds `esp-idf-sys` from scratch when feature flags change, which re-runs the CMake dep check (see above). Initial WiFi build is ~10 minutes; subsequent incremental builds are seconds.
