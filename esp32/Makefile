# Wolf CWL OpenTherm Controller - Rust Firmware
# ==============================================

# Configuration
# PORT is optional: leave unset to let espflash/esptool auto-detect the USB
# serial port (they'll prompt interactively if more than one is present).
# Override with `make flash PORT=/dev/cu.wchusbserial8310` when needed.
PORT ?=
PORT_FLAG := $(if $(PORT),-p $(PORT),)
# Defaults match the populated prototype board: 0.96" SSD1306 (no display-sh1106),
# OT via the ext route (GPIO36/4) — works whether the on-board OT circuit or J5
# DIYless breakout drives the lines. simulate-ot is bench-safe; `prod` swaps it.
FEATURES ?= ethernet,simulate-ot,ot-ext

# Load WiFi credentials from .env (used by dev-wifi / prod-wifi). The dash on
# -include means "don't error if the file is missing" — only the wifi targets
# require it. Variables are exported so build.rs can read them.
-include .env
export WIFI_SSID WIFI_PASSWORD

# Paths
WEB_SRC := web
WEB_OUT := data
PARTITIONS := partitions.csv
SPIFFS_IMG := /tmp/wolf-cwl-spiffs.bin
SPIFFS_SIZE := 0xF0000
SPIFFS_OFFSET := 0x310000
SPIFFSGEN := .embuild/espressif/esp-idf/v5.2.3/components/spiffs/spiffsgen.py
ESPTOOL_PY := .embuild/espressif/python_env/idf5.2_py3.14_env/bin/python3

# Over-the-air update via the device's web server (POST /api/ota/upload). No USB
# needed — the running device writes the app image to its spare OTA partition
# and reboots into it. Override per-invocation, e.g.:
#   make ota-prod OTA_HOST=10.10.2.60 OTA_PASS=secret
OTA_HOST ?= 10.10.2.60
OTA_USER ?= admin
OTA_PASS ?= admin
OTA_IMG := /tmp/wolf-cwl-app.bin
OTA_COOKIE := /tmp/wolf-cwl-ota-cookie.txt

# Colors
CYAN := \033[36m
GREEN := \033[32m
NC := \033[0m

.PHONY: build flash monitor flash-monitor dev prod dev-wifi prod-wifi check-wifi-env build-web build-fs flash-fs flash-all ota ota-prod ota-fs ota-all ota-all-prod clean help setup fix-python-env

help:
	@echo "$(CYAN)Wolf CWL Rust Firmware$(NC)"
	@echo ""
	@echo "  make setup          - First-time: install ldproxy + espflash, patch Python env"
	@echo "  make dev            - Build, flash and monitor (simulation mode, ethernet)"
	@echo "  make prod           - Build, flash and monitor (real OpenTherm, ethernet)"
	@echo "  make dev-wifi       - Build, flash and monitor (simulation mode, wifi from .env)"
	@echo "  make prod-wifi      - Build, flash and monitor (real OpenTherm, wifi from .env)"
	@echo "  make build          - Build firmware"
	@echo "  make flash          - Build and flash firmware via USB"
	@echo "  make monitor        - Open serial monitor"
	@echo "  make build-web      - Build web UI"
	@echo "  make build-fs       - Build web UI + create SPIFFS image"
	@echo "  make flash-fs       - Flash SPIFFS image"
	@echo "  make flash-all      - Flash firmware + SPIFFS (preserves NVS)"
	@echo "  make flash-init     - First-time: flash with partition table (erases NVS!)"
	@echo "  make ota            - Build + upload firmware over the air (uses default features)"
	@echo "  make ota-prod       - Build + upload firmware OTA (real OpenTherm, ethernet)"
	@echo "  make ota-fs         - Upload web UI (SPIFFS) OTA only (reboot to mount)"
	@echo "  make ota-all        - Upload firmware + web UI OTA (uses default features)"
	@echo "  make ota-all-prod   - Upload firmware + web UI OTA (real OpenTherm, ethernet)"
	@echo "  make clean          - Clean build artifacts"
	@echo ""
	@echo "Features: ethernet,simulate-ot,ot-ext (default — 0.96\" SSD1306, J5/UEXT OT pins)"
	@echo "WiFi targets read WIFI_SSID/WIFI_PASSWORD from esp32/.env (see .env.example)"
	@echo ""
	@echo "Prerequisites (one-time, outside this Makefile):"
	@echo "  - Rust + esp toolchain via espup (creates ~/export-esp.sh)"
	@echo "  - 'source ~/export-esp.sh' in your shell before running make"

setup:
	@echo "$(CYAN)Installing ldproxy + espflash (idempotent)...$(NC)"
	cargo install ldproxy espflash
	@$(MAKE) --no-print-directory fix-python-env
	@echo "$(GREEN)Setup complete.$(NC)"
	@echo "Make sure 'source ~/export-esp.sh' is in your shell rc, then try 'make dev-wifi'."

# Python 3.9's importlib.metadata.version('ruamel.yaml') fails to find dist-info
# directories that pip writes with underscores (ruamel_yaml-*.dist-info). The
# ESP-IDF dep check then aborts with exit 255. Symlink the dotted names so the
# check passes. Idempotent; cheap; run before every build so an embuild reinstall
# can't break us silently.
fix-python-env:
	@VENV=".embuild/espressif/python_env/idf5.2_py3.9_env/lib/python3.9/site-packages"; \
	if [ -d "$$VENV" ]; then \
		for d in "$$VENV"/ruamel_yaml-*.dist-info "$$VENV"/ruamel_yaml_clib-*.dist-info; do \
			[ -d "$$d" ] || continue; \
			base=$$(basename "$$d"); \
			target=$$(echo "$$base" | sed -e 's/^ruamel_yaml_clib/ruamel.yaml.clib/' -e 's/^ruamel_yaml/ruamel.yaml/'); \
			if [ "$$base" != "$$target" ] && [ ! -e "$$VENV/$$target" ]; then \
				ln -s "$$base" "$$VENV/$$target"; \
			fi; \
		done; \
	fi

build: fix-python-env
	@echo "$(CYAN)Building firmware (features: $(FEATURES))...$(NC)"
	cargo build --release --features "$(FEATURES)"
	@echo "$(GREEN)Build complete$(NC)"

flash: build
	@echo "$(CYAN)Flashing firmware...$(NC)"
	espflash flash $(PORT_FLAG) --partition-table $(PARTITIONS) target/xtensa-esp32-espidf/release/wolf-cwl
	@echo "$(GREEN)Flash complete$(NC)"

monitor:
	espflash monitor $(PORT_FLAG)

flash-monitor: build
	espflash flash --monitor $(PORT_FLAG) --partition-table $(PARTITIONS) target/xtensa-esp32-espidf/release/wolf-cwl

dev: flash-monitor

prod:
	$(MAKE) flash-monitor FEATURES="ethernet,ot-ext"

check-wifi-env:
	@if [ -z "$$WIFI_SSID" ]; then \
		echo "Error: WIFI_SSID not set. Copy .env.example to .env and fill in your credentials."; \
		exit 1; \
	fi

dev-wifi: check-wifi-env
	$(MAKE) flash-monitor FEATURES="wifi,simulate-ot,ot-ext"

prod-wifi: check-wifi-env
	$(MAKE) flash-monitor FEATURES="wifi,ot-ext"

# First-time setup: flash with partition table (erases NVS!)
flash-init: build
	@echo "$(CYAN)Flashing firmware + partition table (NVS will be erased)...$(NC)"
	espflash flash $(PORT_FLAG) --partition-table $(PARTITIONS) target/xtensa-esp32-espidf/release/wolf-cwl
	@echo "$(GREEN)Flash complete$(NC)"

build-web:
	@echo "$(CYAN)Building web UI...$(NC)"
	cd $(WEB_SRC) && npm run build
	@echo "$(GREEN)Web UI built to $(WEB_OUT)$(NC)"

build-fs: build-web
	@echo "$(CYAN)Creating SPIFFS image...$(NC)"
	$(ESPTOOL_PY) $(SPIFFSGEN) $(SPIFFS_SIZE) $(WEB_OUT) $(SPIFFS_IMG)
	@echo "$(GREEN)SPIFFS image: $(SPIFFS_IMG)$(NC)"

flash-fs: build-fs
	@echo "$(CYAN)Flashing SPIFFS...$(NC)"
	$(ESPTOOL_PY) -m esptool --chip esp32 $(PORT_FLAG) write_flash $(SPIFFS_OFFSET) $(SPIFFS_IMG)
	@echo "$(GREEN)SPIFFS flashed$(NC)"

flash-all: build build-fs
	@echo "$(CYAN)Flashing firmware + SPIFFS...$(NC)"
	espflash flash $(PORT_FLAG) --partition-table $(PARTITIONS) target/xtensa-esp32-espidf/release/wolf-cwl
	$(ESPTOOL_PY) -m esptool --chip esp32 $(PORT_FLAG) write_flash $(SPIFFS_OFFSET) $(SPIFFS_IMG)
	@echo "$(GREEN)All flashed$(NC)"
	espflash monitor $(PORT_FLAG)

# Over-the-air update — no USB. Generates a raw app image and POSTs it to the
# running device, which reboots into it. NOTE: uses the default $(FEATURES)
# (simulation); for the productive device use `ota-prod`.
ota: build
	@echo "$(CYAN)Generating app image...$(NC)"
	espflash save-image --chip esp32 target/xtensa-esp32-espidf/release/wolf-cwl $(OTA_IMG)
	@echo "$(CYAN)Logging in to $(OTA_HOST)...$(NC)"
	@curl -fsS -c $(OTA_COOKIE) -X POST http://$(OTA_HOST)/api/login \
		-H "Content-Type: application/json" \
		-d '{"username":"$(OTA_USER)","password":"$(OTA_PASS)"}' >/dev/null
	@echo "$(CYAN)Uploading firmware to $(OTA_HOST) (device will reboot)...$(NC)"
	@curl -fsS -b $(OTA_COOKIE) -X POST http://$(OTA_HOST)/api/ota/upload \
		--data-binary @$(OTA_IMG) -H "Content-Type: application/octet-stream"
	@echo ""
	@echo "$(GREEN)OTA upload complete — device rebooting$(NC)"

ota-prod:
	$(MAKE) ota FEATURES="ethernet,ot-ext"

# OTA the web UI (SPIFFS) only — no USB. Builds the web bundle + SPIFFS image and
# POSTs it to /api/ota/fs. The device does NOT reboot on its own here, so the new
# UI is mounted on the next reboot (e.g. a following firmware OTA or a power cycle).
ota-fs: build-fs
	@echo "$(CYAN)Logging in to $(OTA_HOST)...$(NC)"
	@curl -fsS -c $(OTA_COOKIE) -X POST http://$(OTA_HOST)/api/login \
		-H "Content-Type: application/json" \
		-d '{"username":"$(OTA_USER)","password":"$(OTA_PASS)"}' >/dev/null
	@echo "$(CYAN)Uploading filesystem to $(OTA_HOST)...$(NC)"
	@curl -fsS -b $(OTA_COOKIE) -X POST http://$(OTA_HOST)/api/ota/fs \
		--data-binary @$(SPIFFS_IMG) -H "Content-Type: application/octet-stream"
	@echo ""
	@echo "$(GREEN)Filesystem OTA complete — reboot to mount the new web UI$(NC)"

# OTA both firmware and web UI in one shot. Order matters: the filesystem is
# uploaded first (no reboot), then the firmware (which reboots), so a single
# reboot at the end activates the new app AND the new web UI together.
ota-all: build build-fs
	@echo "$(CYAN)Generating app image...$(NC)"
	espflash save-image --chip esp32 target/xtensa-esp32-espidf/release/wolf-cwl $(OTA_IMG)
	@echo "$(CYAN)Logging in to $(OTA_HOST)...$(NC)"
	@curl -fsS -c $(OTA_COOKIE) -X POST http://$(OTA_HOST)/api/login \
		-H "Content-Type: application/json" \
		-d '{"username":"$(OTA_USER)","password":"$(OTA_PASS)"}' >/dev/null
	@echo "$(CYAN)Uploading filesystem (no reboot yet)...$(NC)"
	@curl -fsS -b $(OTA_COOKIE) -X POST http://$(OTA_HOST)/api/ota/fs \
		--data-binary @$(SPIFFS_IMG) -H "Content-Type: application/octet-stream"
	@echo ""
	@echo "$(CYAN)Uploading firmware to $(OTA_HOST) (device will reboot)...$(NC)"
	@curl -fsS -b $(OTA_COOKIE) -X POST http://$(OTA_HOST)/api/ota/upload \
		--data-binary @$(OTA_IMG) -H "Content-Type: application/octet-stream"
	@echo ""
	@echo "$(GREEN)OTA (firmware + filesystem) complete — device rebooting$(NC)"

ota-all-prod:
	$(MAKE) ota-all FEATURES="ethernet,ot-ext"

clean:
	cargo clean
	rm -f $(SPIFFS_IMG)
