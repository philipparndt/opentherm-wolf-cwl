# Wolf CWL OpenTherm Controller

ESP32 firmware for controlling a Wolf CWL 300 ventilation unit via OpenTherm protocol, using a DIYLess Master OpenTherm Shield.

Replaces the BM remote control. Provides MQTT integration for Home Assistant, a local OLED display with rotary encoder, and a web UI for configuration.

## Hardware

### Components

| Component | Description |
|-----------|-------------|
| ESP32 dev board | e.g. WeMos D1 Mini ESP32, NodeMCU-32S, or similar |
| [DIYLess Master OpenTherm Shield](https://diyless.com/product/master-opentherm-shield) | OpenTherm electrical interface (voltage modulation / current detection) |
| AZDelivery SH1106 1.3" OLED | 128x64 I2C display for local status and control |
| KY-040 Rotary Encoder | Page navigation and ventilation level control |

### Wiring

The DIYLess shield stacks on top of the ESP32. The OLED and encoder connect via jumper wires.

| Signal | ESP32 GPIO | Component |
|--------|-----------|-----------|
| OT Input | 21 | DIYLess Shield (IN) |
| OT Output | 22 | DIYLess Shield (OUT) |
| I2C SDA | 19 | OLED Display (SDA) |
| I2C SCL | 23 | OLED Display (SCL) |
| Encoder CLK | 16 | KY-040 (CLK) |
| Encoder DT | 17 | KY-040 (DT) |
| Encoder SW | 5 | KY-040 (SW / Button) |
| Status LED | 2 | Built-in LED (on most ESP32 boards) |

All pins are configurable via the web UI after first boot.

### OpenTherm Connection

Connect the DIYLess shield's OpenTherm terminals to the Wolf CWL's OpenTherm bus — the same two wires where the BM remote was connected. OpenTherm is polarity-independent.

**Important:** Only one master can be on the OpenTherm bus. Disconnect the BM remote before connecting the ESP32.

## Software Setup

### Prerequisites

- [PlatformIO](https://platformio.org/) (CLI or VS Code extension)
- [Node.js](https://nodejs.org/) 18+ (for building the web UI)
- Python 3.10-3.12 (for PlatformIO)

### Build

```bash
# Set up PlatformIO (if needed)
make setup-venv

# Install web UI dependencies (first time only)
make install-deps

# Build everything
make build

# Or build components separately
make build-firmware    # Firmware only
make build-web         # Web UI only
make build-fs          # LittleFS filesystem image
```

### Flash via USB

```bash
# Upload firmware
make upload

# Upload web UI filesystem
make upload-fs

# Upload both
make upload-all
```

### Serial Monitor

```bash
make monitor
```

## First-Run Configuration

1. **Power on** the ESP32 — it starts in AP mode since no WiFi is configured
2. **Connect to WiFi** `WolfCWL-XXXX` (password: `wolfcwl123`)
3. **Open** `http://192.168.4.1` in your browser
4. **Enter** your WiFi credentials and MQTT server details
5. **Save** — the ESP32 reboots and connects to your network
6. **Access** the web UI at `http://wolf-cwl.local` (or the device's IP address)
7. **Login** with default credentials: `admin` / `admin` (change in Settings)

### Factory Reset

Hold the encoder button for 10 seconds during boot to clear all settings and return to AP mode.

## OTA Updates

After initial USB flash, update firmware over WiFi:

```bash
# Build and deploy via OTA
make deploy

# Or deploy firmware only
make deploy-firmware
```

Or upload a `.bin` file via the web UI: System tab → Firmware Update.

## Web UI

Access at `http://wolf-cwl.local` (mDNS) or the device's IP address.

| Tab | Description |
|-----|-------------|
| **Status** | Current ventilation level, temperatures, fault/filter/bypass status. Click level buttons to change. |
| **Schedules** | Time-based ventilation schedules (up to 8) and summer mode (bypass) date range. |
| **Settings** | WiFi, MQTT, and web UI credentials. |
| **System** | Firmware version, uptime, heap usage, WiFi RSSI, OTA firmware upload. |

## OLED Display & Encoder

### Pages (rotate encoder to navigate)

| Page | Content |
|------|---------|
| **Home** | Ventilation level (large), relative %, summer/winter mode. `S` = schedule active, `M` = manual override. |
| **Temperatures** | Supply inlet, exhaust inlet, outdoor (from TSP). |
| **Status** | Fault, filter, summer/winter mode, airflow m³/h. |
| **System** | WiFi RSSI, IP, MQTT state, uptime, reboot reason, crash count. |

### Controls

- **Rotate** — switch pages
- **Press** (on home page) — enter edit mode to change ventilation level
- **Rotate** (in edit mode) — cycle through Off / Reduced / Normal / Party
- **Press** (in edit mode) — confirm and apply
- **10s timeout** — edit mode exits without applying

## MQTT Topics

Base topic is configurable (default: `wolf-cwl`).

### Sensor Data (published every ~11s)

| Topic | Value | Description |
|-------|-------|-------------|
| `wolf-cwl/ventilation/level` | `0`-`3` | Current ventilation level |
| `wolf-cwl/ventilation/level_name` | `Off`/`Reduced`/`Normal`/`Party` | Level name |
| `wolf-cwl/ventilation/relative` | `0`-`100` | Relative ventilation % |
| `wolf-cwl/temperature/supply_inlet` | °C | Fresh air intake temperature |
| `wolf-cwl/temperature/exhaust_inlet` | °C | Room exhaust temperature |
| `wolf-cwl/temperature/atmospheric` | °C | Outdoor temperature (from TSP) |
| `wolf-cwl/temperature/indoor` | °C | Indoor temperature (from TSP) |
| `wolf-cwl/status/fault` | `0`/`1` | Fault indication |
| `wolf-cwl/status/filter` | `0`/`1` | Filter needs replacement |
| `wolf-cwl/status/bypass` | `0`/`1` | Bypass/ventilation active |
| `wolf-cwl/airflow/current_volume` | m³/h | Current airflow volume |
| `wolf-cwl/pressure/input_duct` | Pa | Input duct pressure |
| `wolf-cwl/pressure/output_duct` | Pa | Output duct pressure |
| `wolf-cwl/status/frost` | `0`/`1` | Frost protection active |

### Commands (subscribe)

| Topic | Payload | Description |
|-------|---------|-------------|
| `wolf-cwl/set/level` | `0`-`3` | Set ventilation level |
| `wolf-cwl/set/bypass` | `0`/`1` | Open/close bypass (summer mode) |
| `wolf-cwl/set/filter_reset` | `1` | Reset filter timer after replacement |

### Schedule State

| Topic | Value | Description |
|-------|-------|-------------|
| `wolf-cwl/schedule/active` | `0`/`1` | Ventilation schedule controlling level |
| `wolf-cwl/schedule/override` | `0`/`1` | Manual override active |
| `wolf-cwl/schedule/entry_name` | text | Active schedule description |
| `wolf-cwl/bypass/mode` | `summer`/`winter` | Current bypass mode |
| `wolf-cwl/bypass/schedule_active` | `0`/`1` | Bypass schedule controlling state |
| `wolf-cwl/bypass/override` | `0`/`1` | Bypass manual override |

### Health (published every 60s)

| Topic | Value | Description |
|-------|-------|-------------|
| `wolf-cwl/health/uptime` | seconds | Uptime since last boot |
| `wolf-cwl/health/reboot_reason` | text | Last reboot reason (Power/SW/Panic/WDT/Brownout) |
| `wolf-cwl/health/crash_count` | count | Abnormal reboot counter |
| `wolf-cwl/health/free_heap` | bytes | Free heap memory |
| `wolf-cwl/health/ot_response_age` | seconds | Seconds since last CWL response |

### Bridge

| Topic | Value | Description |
|-------|-------|-------------|
| `wolf-cwl/bridge/state` | `online`/`offline` | LWT — broker publishes `offline` on disconnect |
| `wolf-cwl/bridge/version` | text | Firmware version |
| `wolf-cwl/bridge/ip` | IP | Device IP address |

## Ventilation Levels

| Level | Name | Relative Ventilation | Typical Airflow |
|-------|------|---------------------|-----------------|
| 0 | Off | 0% | 0 m³/h |
| 1 | Reduced | 51% | ~100 m³/h |
| 2 | Normal | 67% | ~130 m³/h |
| 3 | Party | 100% | ~195 m³/h |

## Watchdog & Reliability

The firmware includes multiple self-healing mechanisms:

- **Hardware watchdog (TWDT)** — 30s timeout, resets if `loop()` hangs
- **OpenTherm liveness** — restarts if CWL stops responding for 5 minutes
- **Heap monitor** — restarts if free heap drops below 20 KB
- **Crash counter** — tracks abnormal reboots in NVS, visible via MQTT and OLED

## Project Structure

```
esp32/
├── src/
│   ├── main.cpp              # Setup + main loop
│   ├── ot_master.cpp/.h      # OpenTherm polling cycle
│   ├── wolf_cwl.cpp/.h       # CWL data model + TSP cache
│   ├── scheduler.cpp/.h      # Ventilation + bypass schedules
│   ├── mqtt_client.cpp/.h    # MQTT publish/subscribe
│   ├── display.cpp/.h        # SH1106 OLED display
│   ├── encoder.cpp/.h        # KY-040 rotary encoder
│   ├── watchdog.cpp/.h       # Watchdog + liveness
│   ├── config_manager.cpp/.h # NVS configuration
│   ├── network.cpp/.h        # WiFi management
│   ├── ap_mode.cpp/.h        # WiFi AP for setup
│   ├── webserver.cpp/.h      # Web server + API
│   ├── status.cpp/.h         # LED + system status
│   └── logging.h             # Logging macros
├── web/                      # Preact web UI
├── include/config.h          # Compile-time defaults
├── platformio.ini            # Build configuration
├── Makefile                  # Build automation
└── partitions_webui.csv      # Flash partition table
```
