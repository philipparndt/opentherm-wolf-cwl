# Wolf CWL Shield PCB — Schematic & 3D Model Generator
# =====================================================

VENV := .venv
PYTHON := $(VENV)/bin/python
PIP := $(VENV)/bin/pip

# Separate venv for 3D models (needs Python 3.12 for CadQuery)
VENV_3D := .venv-3d
PYTHON_3D := $(VENV_3D)/bin/python
PIP_3D := $(VENV_3D)/bin/pip
PYTHON3_12 := /opt/homebrew/bin/python3.12

# KiCad library paths (macOS KiCad 10)
export KICAD9_SYMBOL_DIR ?= /Applications/KiCad/KiCad.app/Contents/SharedSupport/symbols
export KICAD_SYMBOL_DIR ?= /Applications/KiCad/KiCad.app/Contents/SharedSupport/symbols
export KICAD9_FOOTPRINT_DIR ?= /Applications/KiCad/KiCad.app/Contents/SharedSupport/footprints
export KICAD_FOOTPRINT_DIR ?= /Applications/KiCad/KiCad.app/Contents/SharedSupport/footprints

.PHONY: generate generate-3d clean setup setup-3d help

help:
	@echo "Wolf CWL Shield PCB"
	@echo ""
	@echo "  make setup       - Create venv and install dependencies"
	@echo "  make generate    - Generate KiCad netlist"
	@echo "  make generate-3d - Generate 3D STEP models (requires Python 3.12)"
	@echo "  make clean       - Remove generated files and venvs"

setup: $(VENV)/bin/activate

$(VENV)/bin/activate: requirements.txt
	python3 -m venv $(VENV)
	$(PIP) install --upgrade pip
	$(PIP) install -r requirements.txt
	touch $(VENV)/bin/activate

setup-3d: $(VENV_3D)/bin/activate

$(VENV_3D)/bin/activate: requirements-3d.txt
	$(PYTHON3_12) -m venv $(VENV_3D)
	$(PIP_3D) install --upgrade pip
	$(PIP_3D) install -r requirements-3d.txt
	touch $(VENV_3D)/bin/activate

generate: setup
	$(PYTHON) generate_schematic.py
	@echo ""
	@echo "Next steps:"
	@echo "  1. Open KiCad → New Project"
	@echo "  2. Open PCB Editor"
	@echo "  3. File → Import Netlist → select wolf-cwl-shield.net"
	@echo "  4. Place components and route traces"

generate-3d: setup-3d
	$(PYTHON_3D) generate_3d_models.py

clean:
	rm -rf $(VENV) $(VENV_3D)
	rm -f wolf-cwl-shield.net
	rm -f *.xml
	rm -rf __pycache__
