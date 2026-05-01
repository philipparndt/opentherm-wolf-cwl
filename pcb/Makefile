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

# KiCad
KICAD_CLI := kicad-cli
KICAD_PROJECT := cwl/cwl.kicad_pcb
FREECADCMD := /Applications/FreeCAD.app/Contents/Resources/bin/freecadcmd

# KiCad library paths (macOS KiCad 10)
export KICAD9_SYMBOL_DIR ?= /Applications/KiCad/KiCad.app/Contents/SharedSupport/symbols
export KICAD_SYMBOL_DIR ?= /Applications/KiCad/KiCad.app/Contents/SharedSupport/symbols
export KICAD9_FOOTPRINT_DIR ?= /Applications/KiCad/KiCad.app/Contents/SharedSupport/footprints
export KICAD_FOOTPRINT_DIR ?= /Applications/KiCad/KiCad.app/Contents/SharedSupport/footprints

.PHONY: generate generate-3d export-stl-full export-stl-board export-stl-parts export-stl-print gerber clean setup setup-3d help

help:
	@echo "Wolf CWL Shield PCB"
	@echo ""
	@echo "  make setup            - Create venv and install dependencies"
	@echo "  make generate         - Generate KiCad netlist"
	@echo "  make generate-3d      - Generate 3D STEP models (requires Python 3.12)"
	@echo "  make export-stl-full  - Export full PCB assembly as STL"
	@echo "  make export-stl-print - Export for 3D printing (floating IDC connector)"
	@echo "  make export-stl-board - Export bare PCB only as STL"
	@echo "  make export-stl-parts - Export components only as STL (no board)"
	@echo "  make gerber           - Export Gerbers + drill files for fabrication"
	@echo "  make clean            - Remove generated files and venvs"

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

export-stl-full:
	@echo "Exporting full PCB assembly..."
	@touch cwl/cwl-assembly.step
	$(KICAD_CLI) pcb export step --force --include-tracks --include-pads --include-silkscreen --include-soldermask --output cwl/cwl-assembly.step $(KICAD_PROJECT)
	$(FREECADCMD) -c "import sys; sys.argv=['','$(CURDIR)/cwl/cwl-assembly.step','$(CURDIR)/cwl/cwl-assembly.stl']; exec(open('$(CURDIR)/step2stl.py').read())" 2>/dev/null
	@rm -f cwl/cwl-assembly.step
	@echo "Exported: cwl/cwl-assembly.stl"

export-stl-print:
	@echo "Exporting for 3D printing (floating IDC connector)..."
	@cp 3dmodels/IDC-Socket_2x05_P2.54mm_Vertical.step 3dmodels/IDC-Socket_2x05_P2.54mm_Vertical.step.bak
	@cp 3dmodels/IDC-Socket_2x05_P2.54mm_Vertical_printable.step 3dmodels/IDC-Socket_2x05_P2.54mm_Vertical.step
	@touch cwl/cwl-assembly-print.step
	$(KICAD_CLI) pcb export step --force --include-tracks --include-pads --include-silkscreen --include-soldermask --output cwl/cwl-assembly-print.step $(KICAD_PROJECT)
	$(FREECADCMD) -c "import sys; sys.argv=['','$(CURDIR)/cwl/cwl-assembly-print.step','$(CURDIR)/cwl/cwl-assembly-print.stl']; exec(open('$(CURDIR)/step2stl.py').read())" 2>/dev/null
	@rm -f cwl/cwl-assembly-print.step
	@mv 3dmodels/IDC-Socket_2x05_P2.54mm_Vertical.step.bak 3dmodels/IDC-Socket_2x05_P2.54mm_Vertical.step
	@echo "Exported: cwl/cwl-assembly-print.stl"

export-stl-board:
	@echo "Exporting bare PCB..."
	@touch cwl/cwl-board.step
	$(KICAD_CLI) pcb export step --force --no-components --include-tracks --include-pads --include-silkscreen --include-soldermask --output cwl/cwl-board.step $(KICAD_PROJECT)
	$(FREECADCMD) -c "import sys; sys.argv=['','$(CURDIR)/cwl/cwl-board.step','$(CURDIR)/cwl/cwl-board.stl']; exec(open('$(CURDIR)/step2stl.py').read())" 2>/dev/null
	@rm -f cwl/cwl-board.step
	@echo "Exported: cwl/cwl-board.stl"

export-stl-parts:
	@echo "Exporting components only..."
	@touch cwl/cwl-parts.step
	$(KICAD_CLI) pcb export step --force --subst-models --no-board-body --output cwl/cwl-parts.step $(KICAD_PROJECT)
	$(FREECADCMD) -c "import sys; sys.argv=['','$(CURDIR)/cwl/cwl-parts.step','$(CURDIR)/cwl/cwl-parts.stl']; exec(open('$(CURDIR)/step2stl.py').read())" 2>/dev/null
	@rm -f cwl/cwl-parts.step
	@echo "Exported: cwl/cwl-parts.stl"

gerber:
	@echo "Exporting Gerbers for fabrication..."
	@mkdir -p cwl/gerber
	$(KICAD_CLI) pcb export gerbers \
		--use-drill-file-origin \
		--no-netlist \
		--subtract-soldermask \
		-l "F.Cu,B.Cu,F.Paste,B.Paste,F.SilkS,B.SilkS,F.Mask,B.Mask,Edge.Cuts" \
		-o cwl/gerber/ \
		$(KICAD_PROJECT)
	$(KICAD_CLI) pcb export drill \
		--format excellon \
		--drill-origin plot \
		--excellon-units mm \
		-o cwl/gerber/ \
		$(KICAD_PROJECT)
	@cd cwl/gerber && zip -q ../cwl-gerber.zip *
	@echo "Exported: cwl/cwl-gerber.zip"

clean:
	rm -rf $(VENV) $(VENV_3D)
	rm -f wolf-cwl-shield.net
	rm -f cwl/cwl-assembly.stl cwl/cwl-board.stl cwl/cwl-parts.stl
	rm -f cwl/cwl-assembly.step cwl/cwl-board.step cwl/cwl-parts.step
	rm -rf cwl/gerber cwl/cwl-gerber.zip
	rm -f *.xml
	rm -rf __pycache__
