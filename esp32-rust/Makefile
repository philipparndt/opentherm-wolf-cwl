# Wolf CWL OpenTherm Controller - Rust Firmware
# ==============================================

# Configuration
PORT ?= /dev/cu.wchusbserial8310
FEATURES ?= ethernet,simulate-ot,ot-uext

# Paths
WEB_SRC := ../esp32/web
WEB_OUT := ../esp32/data
PARTITIONS := partitions.csv
SPIFFS_IMG := /tmp/wolf-cwl-spiffs.bin
SPIFFS_SIZE := 0xB0000
SPIFFS_OFFSET := 0x350000
SPIFFSGEN := .embuild/espressif/esp-idf/v5.2.3/components/spiffs/spiffsgen.py
ESPTOOL_PY := .embuild/espressif/python_env/idf5.2_py3.14_env/bin/python3

# Colors
CYAN := \033[36m
GREEN := \033[32m
NC := \033[0m

.PHONY: build flash monitor flash-monitor dev build-web build-fs flash-fs flash-all clean help

help:
	@echo "$(CYAN)Wolf CWL Rust Firmware$(NC)"
	@echo ""
	@echo "  make dev            - Build, flash and monitor (simulation mode)"
	@echo "  make build          - Build firmware"
	@echo "  make flash          - Build and flash firmware via USB"
	@echo "  make monitor        - Open serial monitor"
	@echo "  make build-web      - Build web UI"
	@echo "  make build-fs       - Build web UI + create SPIFFS image"
	@echo "  make flash-fs       - Flash SPIFFS image"
	@echo "  make flash-all      - Flash firmware + SPIFFS (preserves NVS)"
	@echo "  make flash-init     - First-time: flash with partition table (erases NVS!)"
	@echo "  make clean          - Clean build artifacts"
	@echo ""
	@echo "Features: ethernet,simulate-ot (default)"

build:
	@echo "$(CYAN)Building firmware (features: $(FEATURES))...$(NC)"
	cargo build --release --features "$(FEATURES)"
	@echo "$(GREEN)Build complete$(NC)"

flash: build
	@echo "$(CYAN)Flashing firmware...$(NC)"
	espflash flash -p $(PORT) target/xtensa-esp32-espidf/release/wolf-cwl
	@echo "$(GREEN)Flash complete$(NC)"

monitor:
	espflash monitor -p $(PORT)

flash-monitor: build
	espflash flash --monitor -p $(PORT) target/xtensa-esp32-espidf/release/wolf-cwl

dev: flash-monitor

# First-time setup: flash with partition table (erases NVS!)
flash-init: build
	@echo "$(CYAN)Flashing firmware + partition table (NVS will be erased)...$(NC)"
	espflash flash -p $(PORT) --partition-table $(PARTITIONS) target/xtensa-esp32-espidf/release/wolf-cwl
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
	$(ESPTOOL_PY) -m esptool --chip esp32 -p $(PORT) write_flash $(SPIFFS_OFFSET) $(SPIFFS_IMG)
	@echo "$(GREEN)SPIFFS flashed$(NC)"

flash-all: build build-fs
	@echo "$(CYAN)Flashing firmware + SPIFFS...$(NC)"
	espflash flash -p $(PORT) target/xtensa-esp32-espidf/release/wolf-cwl
	$(ESPTOOL_PY) -m esptool --chip esp32 -p $(PORT) write_flash $(SPIFFS_OFFSET) $(SPIFFS_IMG)
	@echo "$(GREEN)All flashed$(NC)"
	espflash monitor -p $(PORT)

clean:
	cargo clean
	rm -f $(SPIFFS_IMG)
