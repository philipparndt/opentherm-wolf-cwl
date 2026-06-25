# Wolf CWL Shield PCB — 3D models & fabrication exports
# ======================================================

# Separate venv for 3D models (needs Python 3.12 for CadQuery)
VENV_3D := .venv-3d
PYTHON_3D := $(VENV_3D)/bin/python
PIP_3D := $(VENV_3D)/bin/pip
PYTHON3_12 := /opt/homebrew/bin/python3.12

# KiCad
KICAD_CLI := kicad-cli
# VARIANT picks which KiCad project to act on. Default cwl/ uses the combined
# OLED footprint (supports both 0.96" and 1.3" modules). cwl-diyless/ uses the
# DIYless D1-mini OT shield and has separate J3 / J3B for the two OLED sizes.
# cwl-compact/ shares the cwl schematic but uses a more compact board layout.
VARIANT ?= cwl
KICAD_PROJECT := $(VARIANT)/cwl.kicad_pcb
FREECADCMD := /Applications/FreeCAD.app/Contents/Resources/bin/freecadcmd

# Display socket substitution (PCBA export only — the board design is untouched).
# The OLED connector (J3) keeps its OLED footprint on the board, which reserves
# the space + mounting holes. In the pcba-full export we relabel J3's BOM line as
# a 1x04 2.54mm female header so a display module can be plugged in, and drop it
# from the CPL: the OLED footprint's origin is ~2-3cm from its header holes, so an
# auto-placed socket would land in the wrong spot — it is hand-soldered instead.
# Set DISPLAY_SOCKET_LCSC to an LCSC part number to populate the BOM's LCSC column.
DISPLAY_REF ?= J3
DISPLAY_SOCKET_COMMENT ?= PinSocket 1x04 P2.54mm (female)
DISPLAY_SOCKET_FOOTPRINT ?= Connector_PinSocket_2.54mm:PinSocket_1x04_P2.54mm_Vertical
# C2718488 = BOOMELE 2.54-1x4P female header (1x04, 2.54mm), JLCPCB parts library.
DISPLAY_SOCKET_LCSC ?= C2718488

# InteractiveHtmlBom: cloned to scripting/plugins/InteractiveHtmlBom (manual install).
# PCM-installed copies live under 3rdparty/plugins/com_github_openscopeproject_InteractiveHtmlBom/plugin/
# — override IBOM_SCRIPT if you used PCM instead.
KICAD_PYTHON ?= /Applications/KiCad/KiCad.app/Contents/Frameworks/Python.framework/Versions/Current/bin/python3
IBOM_SCRIPT ?= $(HOME)/Documents/KiCad/10.0/scripting/plugins/InteractiveHtmlBom/InteractiveHtmlBom/generate_interactive_bom.py

# KiCad library paths (macOS KiCad 10)
export KICAD9_3DMODEL_DIR ?= /Applications/KiCad/KiCad.app/Contents/SharedSupport/3dmodels
export KICAD_3DMODEL_DIR ?= /Applications/KiCad/KiCad.app/Contents/SharedSupport/3dmodels
export KICAD10_3DMODEL_DIR ?= /Applications/KiCad/KiCad.app/Contents/SharedSupport/3dmodels

.PHONY: generate-3d export-stl-full export-stl-board export-stl-parts export-stl-parts-0.96 export-stl-parts-1.3 export-stl-print gerber plot-pdf pcba pcba-full ibom clean setup-3d help

help:
	@echo "Wolf CWL Shield PCB"
	@echo ""
	@echo "  make generate-3d      - Generate 3D STEP models (requires Python 3.12)"
	@echo "  make export-stl-full  - Export full PCB assembly as STL"
	@echo "  make export-stl-print - Export for 3D printing (floating IDC connector)"
	@echo "  make export-stl-board - Export bare PCB only as STL"
	@echo "  make export-stl-parts - Export components only as STL (no board)"
	@echo "  make export-stl-parts-0.96 - Export components only with 0.96\" OLED (cwl-diyless only — excludes J3B)"
	@echo "  make export-stl-parts-1.3  - Export components only with 1.3\" OLED  (cwl-diyless only — excludes J3)"
	@echo "  make gerber           - Export Gerbers + drill files for fabrication"
	@echo "  make plot-pdf         - Plot fabrication / assembly PDFs (front + back)"
	@echo "  make pcba             - Export JLCPCB PCBA bundle, SMD only (gerber + BOM + CPL)"
	@echo "  make pcba-full        - Export JLCPCB PCBA bundle including through-hole parts"
	@echo "  make ibom             - Generate Interactive HTML BOM (needs InteractiveHtmlBom plugin via PCM)"
	@echo "  make clean            - Remove generated files and venv"
	@echo ""
	@echo "  Variants:"
	@echo "    VARIANT=cwl         — main board (combined 0.96\" + 1.3\" OLED footprint, on-board OT) [default]"
	@echo "    VARIANT=cwl-compact — compact board layout (same schematic as cwl)"
	@echo "    VARIANT=cwl-diyless — DIYless D1-mini OT shield + two separate OLED footprints"

setup-3d: $(VENV_3D)/bin/activate

$(VENV_3D)/bin/activate: requirements-3d.txt
	$(PYTHON3_12) -m venv $(VENV_3D)
	$(PIP_3D) install --upgrade pip
	$(PIP_3D) install -r requirements-3d.txt
	touch $(VENV_3D)/bin/activate

generate-3d: setup-3d
	$(PYTHON_3D) generate_3d_models.py

export-stl-full:
	@echo "Exporting full PCB assembly..."
	@touch $(VARIANT)/cwl-assembly.step
	$(KICAD_CLI) pcb export step --force --include-tracks --include-pads --include-silkscreen --include-soldermask --output $(VARIANT)/cwl-assembly.step $(KICAD_PROJECT)
	$(FREECADCMD) -c "import sys; sys.argv=['','$(CURDIR)/$(VARIANT)/cwl-assembly.step','$(CURDIR)/$(VARIANT)/cwl-assembly.stl']; exec(open('$(CURDIR)/step2stl.py').read())" 2>/dev/null
	@rm -f $(VARIANT)/cwl-assembly.step
	@echo "Exported: $(VARIANT)/cwl-assembly.stl"

export-stl-print:
	@echo "Exporting for 3D printing (floating IDC connector)..."
	@cp 3dmodels/IDC-Socket_2x05_P2.54mm_Vertical.step 3dmodels/IDC-Socket_2x05_P2.54mm_Vertical.step.bak
	@cp 3dmodels/IDC-Socket_2x05_P2.54mm_Vertical_printable.step 3dmodels/IDC-Socket_2x05_P2.54mm_Vertical.step
	@touch $(VARIANT)/cwl-assembly-print.step
	$(KICAD_CLI) pcb export step --force --include-tracks --include-pads --include-silkscreen --include-soldermask --output $(VARIANT)/cwl-assembly-print.step $(KICAD_PROJECT)
	$(FREECADCMD) -c "import sys; sys.argv=['','$(CURDIR)/$(VARIANT)/cwl-assembly-print.step','$(CURDIR)/$(VARIANT)/cwl-assembly-print.stl']; exec(open('$(CURDIR)/step2stl.py').read())" 2>/dev/null
	@rm -f $(VARIANT)/cwl-assembly-print.step
	@mv 3dmodels/IDC-Socket_2x05_P2.54mm_Vertical.step.bak 3dmodels/IDC-Socket_2x05_P2.54mm_Vertical.step
	@echo "Exported: $(VARIANT)/cwl-assembly-print.stl"

export-stl-board:
	@echo "Exporting bare PCB..."
	@touch $(VARIANT)/cwl-board.step
	$(KICAD_CLI) pcb export step --force --no-components --include-tracks --include-pads --include-silkscreen --include-soldermask --output $(VARIANT)/cwl-board.step $(KICAD_PROJECT)
	$(FREECADCMD) -c "import sys; sys.argv=['','$(CURDIR)/$(VARIANT)/cwl-board.step','$(CURDIR)/$(VARIANT)/cwl-board.stl']; exec(open('$(CURDIR)/step2stl.py').read())" 2>/dev/null
	@rm -f $(VARIANT)/cwl-board.step
	@echo "Exported: $(VARIANT)/cwl-board.stl"

export-stl-parts:
	@echo "Exporting components only..."
	@touch $(VARIANT)/cwl-parts.step
	$(KICAD_CLI) pcb export step --force --subst-models --no-board-body --output $(VARIANT)/cwl-parts.step $(KICAD_PROJECT)
	$(FREECADCMD) -c "import sys; sys.argv=['','$(CURDIR)/$(VARIANT)/cwl-parts.step','$(CURDIR)/$(VARIANT)/cwl-parts.stl']; exec(open('$(CURDIR)/step2stl.py').read())" 2>/dev/null
	@rm -f $(VARIANT)/cwl-parts.step
	@echo "Exported: $(VARIANT)/cwl-parts.stl"

# cwl-diyless has both OLED footprints (J3=0.96", J3B=1.3"). These two targets
# export a parts-only STL with one of them filtered out, so each variant shows
# just the OLED you populate. The include list is extracted from the .kicad_pcb
# so it stays in sync if you add or remove components later.
export-stl-parts-0.96:
	@echo "Exporting components (0.96\" OLED, excluding J3B)..."
	@refs=`grep -oE 'property "Reference" "[^"]+"' $(KICAD_PROJECT) | sed -E 's/.*"([^"]+)"/\1/' | sort -u | grep -v '^J3B$$' | paste -sd, -`; \
	if [ -z "$$refs" ]; then echo "No footprints in $(KICAD_PROJECT)."; exit 1; fi; \
	echo "Including refs: $$refs"; \
	touch $(VARIANT)/cwl-parts-0.96.step; \
	$(KICAD_CLI) pcb export step --force --subst-models --no-board-body --component-filter "$$refs" --output $(VARIANT)/cwl-parts-0.96.step $(KICAD_PROJECT); \
	$(FREECADCMD) -c "import sys; sys.argv=['','$(CURDIR)/$(VARIANT)/cwl-parts-0.96.step','$(CURDIR)/$(VARIANT)/cwl-parts-0.96.stl']; exec(open('$(CURDIR)/step2stl.py').read())" 2>/dev/null; \
	rm -f $(VARIANT)/cwl-parts-0.96.step; \
	echo "Exported: $(VARIANT)/cwl-parts-0.96.stl"

export-stl-parts-1.3:
	@echo "Exporting components (1.3\" OLED, excluding J3)..."
	@refs=`grep -oE 'property "Reference" "[^"]+"' $(KICAD_PROJECT) | sed -E 's/.*"([^"]+)"/\1/' | sort -u | grep -v '^J3$$' | paste -sd, -`; \
	if [ -z "$$refs" ]; then echo "No footprints in $(KICAD_PROJECT)."; exit 1; fi; \
	echo "Including refs: $$refs"; \
	touch $(VARIANT)/cwl-parts-1.3.step; \
	$(KICAD_CLI) pcb export step --force --subst-models --no-board-body --component-filter "$$refs" --output $(VARIANT)/cwl-parts-1.3.step $(KICAD_PROJECT); \
	$(FREECADCMD) -c "import sys; sys.argv=['','$(CURDIR)/$(VARIANT)/cwl-parts-1.3.step','$(CURDIR)/$(VARIANT)/cwl-parts-1.3.stl']; exec(open('$(CURDIR)/step2stl.py').read())" 2>/dev/null; \
	rm -f $(VARIANT)/cwl-parts-1.3.step; \
	echo "Exported: $(VARIANT)/cwl-parts-1.3.stl"

gerber:
	@echo "Exporting Gerbers for fabrication..."
	@mkdir -p $(VARIANT)/gerber
	$(KICAD_CLI) pcb export gerbers \
		--use-drill-file-origin \
		--no-netlist \
		--subtract-soldermask \
		-l "F.Cu,B.Cu,F.Paste,B.Paste,F.SilkS,B.SilkS,F.Mask,B.Mask,Edge.Cuts" \
		-o $(VARIANT)/gerber/ \
		$(KICAD_PROJECT)
	$(KICAD_CLI) pcb export drill \
		--format excellon \
		--drill-origin plot \
		--excellon-units mm \
		-o $(VARIANT)/gerber/ \
		$(KICAD_PROJECT)
	@cd $(VARIANT)/gerber && zip -q ../cwl-gerber.zip *
	@echo "Exported: $(VARIANT)/cwl-gerber.zip"

plot-pdf:
	@echo "Plotting assembly PDFs..."
	@mkdir -p $(VARIANT)/plot
	$(KICAD_CLI) pcb export pdf \
		--layers "F.Fab,F.Silkscreen,Edge.Cuts" \
		--mode-single \
		--include-border-title \
		-o $(VARIANT)/plot/cwl-assembly-front.pdf \
		$(KICAD_PROJECT)
	$(KICAD_CLI) pcb export pdf \
		--layers "B.Fab,B.Silkscreen,Edge.Cuts" \
		--mode-single \
		--mirror \
		--include-border-title \
		-o $(VARIANT)/plot/cwl-assembly-back.pdf \
		$(KICAD_PROJECT)
	@echo "Exported: $(VARIANT)/plot/cwl-assembly-front.pdf"
	@echo "Exported: $(VARIANT)/plot/cwl-assembly-back.pdf"

# JLCPCB PCBA bundle (SMD parts only — through-hole parts are hand-assembled).
# Produces in $(VARIANT)/pcba/:
#   cwl-bom.csv      — JLCPCB-format BOM (Comment, Designator, Footprint, LCSC Part #)
#   cwl-cpl.csv      — JLCPCB-format pick & place (Designator, Mid X, Mid Y, Layer, Rotation)
#   cwl-gerber.zip   — copied from the gerber target
# Add an "LCSC" field to each SMD schematic symbol with its LCSC part number so
# the BOM's LCSC Part # column is populated. DNP and through-hole parts are
# excluded from both BOM and CPL.
pcba: gerber
	@echo "Exporting JLCPCB PCBA files (SMD only — THT assembled by hand)..."
	@mkdir -p $(VARIANT)/pcba
	$(KICAD_CLI) pcb export pos \
		--format csv \
		--units mm \
		--use-drill-file-origin \
		--side both \
		--exclude-fp-th \
		--exclude-dnp \
		-o $(VARIANT)/pcba/cwl-cpl-raw.csv \
		$(KICAD_PROJECT)
	@# Index PosX/PosY/Rot/Side from the right: a footprint Package name may
	@# contain a comma (e.g. Phoenix "MKDS-1,5-...") which would shift columns
	@# counted from the left. The trailing position fields never contain commas.
	@# Skip mounting holes (MH*) — not assembled parts. (Solder bridges SB* are
	@# already excluded by their exclude_from_pos_files footprint attribute.)
	@awk -F',' 'BEGIN{OFS=","} \
		NR==1 {print "Designator,Mid X,Mid Y,Layer,Rotation"; next} \
		$$1 ~ /^"?MH[0-9]/ {next} \
		{print $$1, $$(NF-3), $$(NF-2), $$NF, $$(NF-1)}' \
		$(VARIANT)/pcba/cwl-cpl-raw.csv > $(VARIANT)/pcba/cwl-cpl.csv
	@rm -f $(VARIANT)/pcba/cwl-cpl-raw.csv
	$(KICAD_CLI) sch export bom \
		--fields "Value,Reference,Footprint,LCSC" \
		--labels "Comment,Designator,Footprint,LCSC Part #" \
		--exclude-dnp \
		-o $(VARIANT)/pcba/cwl-bom-raw.csv \
		$(VARIANT)/cwl.kicad_sch
	@# Keep only BOM rows whose Designator appears in the CPL (i.e. SMD parts).
	@tail -n +2 $(VARIANT)/pcba/cwl-cpl.csv | cut -d',' -f1 | tr -d '"' | sort -u > $(VARIANT)/pcba/.smd-refs
	@awk -F',' 'NR==FNR {smd[$$1]=1; next} \
		FNR==1 {print; next} \
		{ref=$$2; gsub(/"/, "", ref); if (ref in smd) print}' \
		$(VARIANT)/pcba/.smd-refs $(VARIANT)/pcba/cwl-bom-raw.csv \
		> $(VARIANT)/pcba/cwl-bom.csv
	@rm -f $(VARIANT)/pcba/cwl-bom-raw.csv $(VARIANT)/pcba/.smd-refs
	@cp $(VARIANT)/cwl-gerber.zip $(VARIANT)/pcba/
	@echo "Exported: $(VARIANT)/pcba/cwl-bom.csv"
	@echo "Exported: $(VARIANT)/pcba/cwl-cpl.csv"
	@echo "Exported: $(VARIANT)/pcba/cwl-gerber.zip"

# JLCPCB PCBA bundle including through-hole parts (full assembly). Same as the
# pcba target but without --exclude-fp-th, so the CPL — and the BOM rows derived
# from it — also cover through-hole components. Outputs to $(VARIANT)/pcba-full/.
# DNP parts are still excluded.
pcba-full: gerber
	@echo "Exporting JLCPCB PCBA files (full — SMD + through-hole)..."
	@mkdir -p $(VARIANT)/pcba-full
	$(KICAD_CLI) pcb export pos \
		--format csv \
		--units mm \
		--use-drill-file-origin \
		--side both \
		--exclude-dnp \
		-o $(VARIANT)/pcba-full/cwl-cpl-raw.csv \
		$(KICAD_PROJECT)
	@# Index PosX/PosY/Rot/Side from the right: a footprint Package name may
	@# contain a comma (e.g. Phoenix "MKDS-1,5-...") which would shift columns
	@# counted from the left. The trailing position fields never contain commas.
	@# Skip mounting holes (MH*) — not assembled parts. (Solder bridges SB* are
	@# already excluded by their exclude_from_pos_files footprint attribute.)
	@# Also skip the display connector ($(DISPLAY_REF)) — hand-soldered, see header.
	@awk -F',' 'BEGIN{OFS=","} \
		NR==1 {print "Designator,Mid X,Mid Y,Layer,Rotation"; next} \
		$$1 ~ /^"?MH[0-9]/ {next} \
		$$1 ~ /^"?$(DISPLAY_REF)"?$$/ {next} \
		{print $$1, $$(NF-3), $$(NF-2), $$NF, $$(NF-1)}' \
		$(VARIANT)/pcba-full/cwl-cpl-raw.csv > $(VARIANT)/pcba-full/cwl-cpl.csv
	@rm -f $(VARIANT)/pcba-full/cwl-cpl-raw.csv
	$(KICAD_CLI) sch export bom \
		--fields "Value,Reference,Footprint,LCSC" \
		--labels "Comment,Designator,Footprint,LCSC Part #" \
		--exclude-dnp \
		-o $(VARIANT)/pcba-full/cwl-bom-raw.csv \
		$(VARIANT)/cwl.kicad_sch
	@# Keep BOM rows whose Designator is in the CPL, plus the hand-soldered display
	@# connector ($(DISPLAY_REF)) which is intentionally not in the CPL.
	@{ tail -n +2 $(VARIANT)/pcba-full/cwl-cpl.csv | cut -d',' -f1 | tr -d '"'; echo "$(DISPLAY_REF)"; } | sort -u > $(VARIANT)/pcba-full/.placed-refs
	@awk -F',' 'NR==FNR {placed[$$1]=1; next} \
		FNR==1 {print; next} \
		{ref=$$2; gsub(/"/, "", ref); if (ref in placed) print}' \
		$(VARIANT)/pcba-full/.placed-refs $(VARIANT)/pcba-full/cwl-bom-raw.csv \
		> $(VARIANT)/pcba-full/cwl-bom.csv
	@# Relabel the display connector's BOM row as the female socket to hand-solder.
	@awk -F',' 'BEGIN{OFS=","} \
		$$2 ~ /^"?$(DISPLAY_REF)"?$$/ {print "\"$(DISPLAY_SOCKET_COMMENT)\"", $$2, "\"$(DISPLAY_SOCKET_FOOTPRINT)\"", "\"$(DISPLAY_SOCKET_LCSC)\""; next} \
		{print}' \
		$(VARIANT)/pcba-full/cwl-bom.csv > $(VARIANT)/pcba-full/cwl-bom.tmp \
		&& mv $(VARIANT)/pcba-full/cwl-bom.tmp $(VARIANT)/pcba-full/cwl-bom.csv
	@rm -f $(VARIANT)/pcba-full/cwl-bom-raw.csv $(VARIANT)/pcba-full/.placed-refs
	@cp $(VARIANT)/cwl-gerber.zip $(VARIANT)/pcba-full/
	@echo "Exported: $(VARIANT)/pcba-full/cwl-bom.csv"
	@echo "Exported: $(VARIANT)/pcba-full/cwl-cpl.csv"
	@echo "Exported: $(VARIANT)/pcba-full/cwl-gerber.zip"

ibom:
	@echo "Generating Interactive HTML BOM..."
	@if [ ! -f "$(IBOM_SCRIPT)" ]; then \
		echo "ERROR: InteractiveHtmlBom not found at:"; \
		echo "  $(IBOM_SCRIPT)"; \
		echo ""; \
		echo "Install via KiCad PCM: PCB Editor → Tools → Plugin and Content Manager → InteractiveHtmlBom."; \
		echo "Or override IBOM_SCRIPT=/path/to/generate_interactive_bom.py."; \
		exit 1; \
	fi
	$(KICAD_PYTHON) $(IBOM_SCRIPT) \
		--no-browser \
		--dest-dir bom \
		--name-format ibom \
		$(KICAD_PROJECT)
	@echo "Exported: $(VARIANT)/bom/ibom.html"

clean:
	rm -rf $(VENV_3D)
	rm -f $(VARIANT)/cwl-assembly.stl $(VARIANT)/cwl-board.stl $(VARIANT)/cwl-parts.stl $(VARIANT)/cwl-parts-0.96.stl $(VARIANT)/cwl-parts-1.3.stl
	rm -f $(VARIANT)/cwl-assembly.step $(VARIANT)/cwl-board.step $(VARIANT)/cwl-parts.step $(VARIANT)/cwl-parts-0.96.step $(VARIANT)/cwl-parts-1.3.step
	rm -rf $(VARIANT)/gerber $(VARIANT)/cwl-gerber.zip
	rm -rf $(VARIANT)/plot $(VARIANT)/bom $(VARIANT)/pcba $(VARIANT)/pcba-full
	rm -rf __pycache__
