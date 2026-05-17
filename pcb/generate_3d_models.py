"""
Generate 3D STEP models for custom PCB components.

Run with: python generate_3d_models.py
Outputs:
  3dmodels/IDC-Socket_2x05_P2.54mm_Vertical.step
  3dmodels/IDC-Socket_2x05_P2.54mm_Vertical_printable.step
  3dmodels/OLED_SH1106_0.96inch.step
  3dmodels/OLED_SH1106_1.3inch.step
  3dmodels/SOP-4_7.5x4.1mm_P2.54mm.step
"""

import cadquery as cq

# When True, generates both normal and printable IDC variants
# Printable version: hollow cavity, no pins (connector floats over ESP32 header)
GENERATE_BOTH = True


def idc_female_socket_2x05(printable=False):
    """
    2x5 IDC female socket, 2.54mm pitch.
    Body: 17.1 x 5.8 x 9.5mm
    Keying bump: 3.7mm wide, 1.3mm standout, 6mm tall from top
    Chamfer: 1.2mm wide, 2.8mm tall on top edges
    Rectangular pin holes

    printable=True: large hollow cavity (for 3D printing over ESP32 header),
                    no pin tails (connector floats, not soldered)
    """
    pitch = 2.54
    cols = 5
    rows = 2

    body_l = 17.1
    body_d = 5.8
    body_h = 9.5

    key_w = 3.7
    key_standout = 1.3
    key_h = 6.0

    chamfer_w = 1.2
    chamfer_h = 2.8

    hole_w = 0.8   # rectangular hole width
    hole_d = 0.8   # rectangular hole depth

    pin_sq = 0.64
    pin_below = 3.0

    cx = (cols - 1) * pitch / 2
    cy = (rows - 1) * pitch / 2

    # Main body — for printable, extend housing down to PCB surface
    base_ext = 0.25 if printable else 0  # extend down to close the gap to PCB
    body = (
        cq.Workplane("XY")
        .box(body_l, body_d, body_h + base_ext, centered=(True, True, False))
        .translate((cx, cy, -base_ext))
    )

    # Chamfer on the two short edges on top (left + right sides)
    body = body.edges(">Z").edges("|Y").chamfer(chamfer_h, chamfer_w)

    # Keying bump — from top, 6mm tall
    bump = (
        cq.Workplane("XY")
        .box(key_w, key_standout, key_h, centered=(True, True, False))
        .translate((cx, cy + body_d / 2 + key_standout / 2, body_h - key_h))
    )
    body = body.union(bump)

    if printable:
        # 3D printing: hollow out the inside to fit over ESP32-POE box header
        cavity_w = (cols - 1) * pitch + 2.0
        cavity_d = (rows - 1) * pitch + 2.0
        cavity = (
            cq.Workplane("XY")
            .workplane(offset=-0.1)
            .center(cx, cy)
            .rect(cavity_w, cavity_d)
            .extrude(body_h + 0.2)
        )
        body = body.cut(cavity)
    else:
        # Accurate model: individual rectangular pin holes
        for col in range(cols):
            for row in range(rows):
                x = col * pitch
                y = row * pitch
                hole = (
                    cq.Workplane("XY")
                    .workplane(offset=-0.1)
                    .center(x, y)
                    .rect(hole_w, hole_d)
                    .extrude(body_h + 0.2)
                )
                body = body.cut(hole)

    # Pin tails below PCB
    if printable:
        # Printable: pins are part of the housing (one solid piece, no gap)
        for col in range(cols):
            for row in range(rows):
                x = col * pitch
                y = row * pitch
                pin = (
                    cq.Workplane("XY")
                    .box(pin_sq, pin_sq, pin_below, centered=(True, True, False))
                    .translate((x, y, -pin_below))
                )
                body = body.union(pin)
        pin_bodies = None
    else:
        # Normal: pins are a separate body (different color in assembly)
        pin_bodies = cq.Workplane("XY")
        for col in range(cols):
            for row in range(rows):
                x = col * pitch
                y = row * pitch
                pin = (
                    cq.Workplane("XY")
                    .box(pin_sq, pin_sq, pin_below, centered=(True, True, False))
                    .translate((x, y, -pin_below))
                )
                pin_bodies = pin_bodies.union(pin)

    # Apply rotation (90° Z) and offset to align with KiCad footprint pads
    def transform(shape):
        return shape.rotateAboutCenter((0, 0, 1), 90).translate((2.5 - 6.3, -10.15 + 3.8, 0.25))

    body = transform(body)
    if pin_bodies:
        pin_bodies = transform(pin_bodies)

    return body, pin_bodies


def oled_sh1106_0_96inch():
    """
    0.96" SH1106 128x64 OLED module with 4-pin I2C header and mounting holes.
    Dimensions matched to the 128x64OLED-MountingHoles footprint.

    Footprint origin is at top-left corner of the board.
    Board: 27.4 x 27.3 mm
    OLED glass: from (0.3, 4.1) to (27.1, 19.9) — 26.8 x 15.8mm
    Pins at: x = 9.78, 12.32, 14.86, 17.4  y = 1.7
    Mounting holes at: (1.9, 1.9), (25.6, 1.8), (1.9, 25.4), (25.5, 25.4)
    """
    # Board dimensions
    board_w = 27.4
    board_h = 27.3
    board_thick = 1.6

    # OLED glass area — centered on the board
    glass_w = 27.3
    glass_h = 19.2
    glass_cx = board_w / 2
    glass_cy = board_h / 2  # centered vertically on board
    glass_thick = 1.2
    glass_bottom = glass_cy + glass_h / 2  # bottom edge of glass in footprint coords

    # Active display area (dark part)
    disp_w = 23.0
    disp_h = 12.5

    # Pin positions (from footprint, KiCad Y is flipped vs 3D)
    pin_positions = [(9.78, 1.7), (12.32, 1.7), (14.86, 1.7), (17.4, 1.7)]
    pin_sq = 0.64
    pin_below = 7.9 - 1.5  # pin length below board
    pin_above = 1.5   # pin extends above board

    # Mounting holes
    mount_holes = [(1.9, 1.9), (25.6, 1.8), (1.9, 25.4), (25.5, 25.4)]
    mount_drill = 2.2

    # FPC connector on bottom edge of glass (ribbon cable to OLED glass)
    fpc_w = 16.0
    fpc_d = 2.0
    fpc_h = 1.0

    # PCB cutout for FPC cable fold
    cutout_w = 20.0
    cutout_d = 3.0

    # KiCad footprint origin is top-left, Y goes down
    # CadQuery: we'll build with origin at footprint origin, Y = KiCad Y (positive = down)
    # But CadQuery Y goes up in 3D, so we negate Y for the 3D model
    # Actually, KiCad 3D viewer uses: X=right, Y=down(into screen), Z=up
    # CadQuery uses: X=right, Y=up, Z=towards viewer
    # The STEP file just needs to match — KiCad handles the mapping

    # Build PCB board (dark blue/black)
    # Origin at (0,0) = top-left of footprint, board extends to (27.4, 27.3)
    # In 3D: center the board, then offset
    pcb = (
        cq.Workplane("XY")
        .box(board_w, board_h, board_thick, centered=(True, True, False))
        .translate((board_w / 2, -board_h / 2, 0))
    )

    # Drill mounting holes
    for mx, my in mount_holes:
        hole = (
            cq.Workplane("XY")
            .workplane(offset=-0.1)
            .center(mx, -my)
            .circle(mount_drill / 2)
            .extrude(board_thick + 0.2)
        )
        pcb = pcb.cut(hole)

    # Drill pin holes
    for px, py in pin_positions:
        hole = (
            cq.Workplane("XY")
            .workplane(offset=-0.1)
            .center(px, -py)
            .circle(0.6)
            .extrude(board_thick + 0.2)
        )
        pcb = pcb.cut(hole)

    # Glass consists of two zones:
    # - Dark zone: 16mm tall (top part, contains the visible display)
    # - Transparent zone: 3.2mm tall (bottom part, FPC connection visible)
    dark_h = 16.0
    transparent_h = 3.2
    # glass_h = dark_h + transparent_h = 19.2mm

    glass_top = glass_cy - glass_h / 2  # top edge of glass in footprint Y

    # Dark part of glass (top 16mm)
    glass_dark = (
        cq.Workplane("XY")
        .box(glass_w, dark_h, glass_thick, centered=(True, True, False))
        .translate((glass_cx, -(glass_top + dark_h / 2), board_thick))
    )

    # Transparent part of glass (bottom 3.2mm)
    glass_transparent = (
        cq.Workplane("XY")
        .box(glass_w, transparent_h, glass_thick, centered=(True, True, False))
        .translate((glass_cx, -(glass_top + dark_h + transparent_h / 2), board_thick))
    )

    # Active display area (centered within the dark zone)
    dark_center_y = glass_top + dark_h / 2
    display = (
        cq.Workplane("XY")
        .box(disp_w, disp_h, 0.1, centered=(True, True, False))
        .translate((glass_cx, -dark_center_y, board_thick + glass_thick))
    )

    # FPC connector (small brown/tan bar at bottom of glass, connecting glass to PCB)
    fpc = (
        cq.Workplane("XY")
        .box(fpc_w, fpc_d, fpc_h, centered=(True, True, False))
        .translate((glass_cx, -(glass_bottom + fpc_d / 2), board_thick))
    )

    # PCB cutout for FPC cable (flush with board edge)
    # l=14.3mm, d=2.7mm, on the border of the PCB
    cutout_w = 14.3
    cutout_d = 2.7
    cutout = (
        cq.Workplane("XY")
        .workplane(offset=-0.1)
        .box(cutout_w, cutout_d, board_thick + 0.2, centered=(True, True, False))
        .translate((glass_cx, -(board_h - cutout_d / 2), 0))
    )
    pcb = pcb.cut(cutout)

    # Pin headers (gold)
    pins = cq.Workplane("XY")
    for px, py in pin_positions:
        pin = (
            cq.Workplane("XY")
            .box(pin_sq, pin_sq, pin_below + board_thick + pin_above, centered=(True, True, False))
            .translate((px, -py, -pin_below))
        )
        pins = pins.union(pin)

    # Pin spacer / distance holder (black plastic block around pins)
    spacer_pitch = 2.54
    spacer_w = (len(pin_positions) - 1) * spacer_pitch + 2.54  # width covering all 4 pins
    spacer_d = 2.54   # depth (single row)
    spacer_h = 2.5    # height
    spacer_cx = (pin_positions[0][0] + pin_positions[-1][0]) / 2
    spacer_cy = pin_positions[0][1]
    spacer = (
        cq.Workplane("XY")
        .box(spacer_w, spacer_d, spacer_h, centered=(True, True, False))
        .translate((spacer_cx, -spacer_cy, -spacer_h))
    )

    # Drill pin holes through spacer
    for px, py in pin_positions:
        hole = (
            cq.Workplane("XY")
            .workplane(offset=-spacer_h - 0.1)
            .center(px, -py)
            .rect(pin_sq + 0.2, pin_sq + 0.2)
            .extrude(spacer_h + 0.2)
        )
        spacer = spacer.cut(hole)

    # Bake in Z offset (2.6mm above shield PCB)
    z_off = 2.6
    pcb = pcb.translate((0, 0, z_off))
    glass_dark = glass_dark.translate((0, 0, z_off))
    glass_transparent = glass_transparent.translate((0, 0, z_off))
    display = display.translate((0, 0, z_off))
    fpc = fpc.translate((0, 0, z_off))
    pins = pins.translate((0, 0, z_off))
    spacer = spacer.translate((0, 0, z_off))

    return pcb, glass_dark, glass_transparent, display, fpc, pins, spacer


def oled_sh1106_1_3inch():
    """
    1.3" SH1106 128x64 OLED module — large variant.

    Board: 35.6 × 33.8 mm. Header on the side where mounting holes are inset
    further from the edge (KiCad-Y high). Mounting holes per oled_1_3.scad.

    Pin order (looking at the front, pad numbering runs right → left along
    the header so VDD ends up on the high-x end):
        pad 1 = VDD (high x), 2 = GND, 3 = SCK(=SCL), 4 = SDA (low x)
    """
    board_w = 35.6
    board_h = 33.8
    board_thick = 1.6

    # Mounting holes (match oled_1_3.scad)
    mount_holes = [(2.5, 2.0), (33.0, 2.0), (2.5, 30.8), (33.0, 30.8)]
    mount_drill = 2.2

    # Header — 4 pins, 2.54mm pitch, centred between upper mounting holes
    # (upper holes at y=30.8, inset 3mm from y=33.8 edge → header lives near that edge)
    pin_pitch = 2.54
    pin_y = 32.1
    pin_x0 = (2.5 + 33.0) / 2 - 1.5 * pin_pitch  # 13.94
    pin_positions = [(pin_x0 + i * pin_pitch, pin_y) for i in range(4)]
    pin_sq = 0.64
    pin_below = 7.9 - 1.5
    pin_above = 1.5

    # Glass area — centred horizontally, FPC end pointed at the header edge
    glass_w = 34.5  # long edge, along board x
    glass_h = 22.5  # short edge, along board y
    # Dark/transparent split: keep the FPC strip ~3.5mm tall, rest is the dark
    # zone where pixels live.
    transparent_h = 3.5
    dark_h = glass_h - transparent_h
    glass_cx = board_w / 2
    # Place the FPC end of the glass so it sits just above the header silkscreen
    # box (which runs y=31.39..32.81); glass_bottom = 29.0 leaves clearance.
    glass_bottom = 29.0
    glass_top = glass_bottom - glass_h
    glass_thick = 1.2

    # Active display area inside the dark zone
    disp_w = 29.8  # long edge, along board x
    disp_h = 15.2  # short edge, along board y

    # FPC at bottom of glass (between glass and header)
    fpc_w = 18.0
    fpc_d = 2.0
    fpc_h = 1.0

    # PCB body
    pcb = (
        cq.Workplane("XY")
        .box(board_w, board_h, board_thick, centered=(True, True, False))
        .translate((board_w / 2, -board_h / 2, 0))
    )

    # Mounting hole drills
    for mx, my in mount_holes:
        hole = (
            cq.Workplane("XY")
            .workplane(offset=-0.1)
            .center(mx, -my)
            .circle(mount_drill / 2)
            .extrude(board_thick + 0.2)
        )
        pcb = pcb.cut(hole)

    # Pin holes
    for px, py in pin_positions:
        hole = (
            cq.Workplane("XY")
            .workplane(offset=-0.1)
            .center(px, -py)
            .circle(0.6)
            .extrude(board_thick + 0.2)
        )
        pcb = pcb.cut(hole)

    # Glass — dark part (where the display sits)
    glass_dark = (
        cq.Workplane("XY")
        .box(glass_w, dark_h, glass_thick, centered=(True, True, False))
        .translate((glass_cx, -(glass_top + dark_h / 2), board_thick))
    )

    # Glass — transparent part (FPC area visible)
    glass_transparent = (
        cq.Workplane("XY")
        .box(glass_w, transparent_h, glass_thick, centered=(True, True, False))
        .translate((glass_cx, -(glass_top + dark_h + transparent_h / 2), board_thick))
    )

    # Active display
    dark_center_y = glass_top + dark_h / 2
    display = (
        cq.Workplane("XY")
        .box(disp_w, disp_h, 0.1, centered=(True, True, False))
        .translate((glass_cx, -dark_center_y, board_thick + glass_thick))
    )

    # FPC ribbon, sitting just below the glass on the PCB
    glass_bottom = glass_top + glass_h
    fpc = (
        cq.Workplane("XY")
        .box(fpc_w, fpc_d, fpc_h, centered=(True, True, False))
        .translate((glass_cx, -(glass_bottom + fpc_d / 2), board_thick))
    )

    # Header pins
    pins = cq.Workplane("XY")
    for px, py in pin_positions:
        pin = (
            cq.Workplane("XY")
            .box(pin_sq, pin_sq, pin_below + board_thick + pin_above, centered=(True, True, False))
            .translate((px, -py, -pin_below))
        )
        pins = pins.union(pin)

    # Pin spacer
    spacer_w = (len(pin_positions) - 1) * pin_pitch + 2.54
    spacer_d = 2.54
    spacer_h = 2.5
    spacer_cx = (pin_positions[0][0] + pin_positions[-1][0]) / 2
    spacer_cy = pin_positions[0][1]
    spacer = (
        cq.Workplane("XY")
        .box(spacer_w, spacer_d, spacer_h, centered=(True, True, False))
        .translate((spacer_cx, -spacer_cy, -spacer_h))
    )
    for px, py in pin_positions:
        hole = (
            cq.Workplane("XY")
            .workplane(offset=-spacer_h - 0.1)
            .center(px, -py)
            .rect(pin_sq + 0.2, pin_sq + 0.2)
            .extrude(spacer_h + 0.2)
        )
        spacer = spacer.cut(hole)

    z_off = 2.6
    pcb = pcb.translate((0, 0, z_off))
    glass_dark = glass_dark.translate((0, 0, z_off))
    glass_transparent = glass_transparent.translate((0, 0, z_off))
    display = display.translate((0, 0, z_off))
    fpc = fpc.translate((0, 0, z_off))
    pins = pins.translate((0, 0, z_off))
    spacer = spacer.translate((0, 0, z_off))

    return pcb, glass_dark, glass_transparent, display, fpc, pins, spacer


def sop4_long_creepage():
    """
    SOP-4 4-pin SMD optocoupler — long-creepage variant.
    Matches KiCad footprint Package_SO:SOP-4_7.5x4.1mm_P2.54mm and the
    Lite-On LTV-817S-B / Sharp PC817X package dimensions.

    Footprint pads sit at (±4.6875, ±1.27). The 7.5 mm and 4.1 mm in the
    footprint name are the BODY dimensions (across × along leads), taken
    directly from the F.Fab polygon: X=-3.75..+3.75, Y=-2.05..+2.05 with
    a 1mm chamfer at the pin-1 corner.

    Coordinate convention: CadQuery/STEP Y axis is the inverse of KiCad
    footprint Y (KiCad PCB editor Y points down, STEP/3D Y points up). So
    KiCad footprint pin 1 at (X=-4.6875, Y=-1.27) lands in this model at
    (X=-4.6875, Y=+1.27).
    """
    # Body — matches F.Fab outline of the KiCad footprint
    body_w = 7.50           # X (across, perpendicular to lead pitch)
    body_l = 4.10           # Y (along lead pitch)
    body_h = 2.10           # Z (height of plastic body)
    body_standoff = 0.10    # gap between body bottom and PCB top
    pin1_chamfer = 1.00     # 1mm corner cut at pin-1 indicator corner

    # Pad centres (from the footprint, copied here as authoritative)
    pad_x = 4.6875
    pad_y = 1.27

    # Lead geometry
    lead_w = 0.45           # along Y (lead-pitch direction)
    lead_t = 0.20           # Z thickness of the lead foil
    body_edge_x = body_w / 2   # 3.75

    # Body footprint as a polygon, matching the F.Fab pin-1 corner chamfer.
    # F.Fab polygon (KiCad footprint coords): (-2.75,-2.05) (3.75,-2.05)
    # (3.75,2.05) (-3.75,2.05) (-3.75,-1.05). After flipping Y for STEP space,
    # the chamfer corner ends up at (X=-3.75, Y=+2.05) — pin 1 side.
    body_pts = [
        (-body_w / 2,          body_l / 2 - pin1_chamfer),  # start of pin-1 chamfer
        (-body_w / 2 + pin1_chamfer, body_l / 2),           # end of pin-1 chamfer
        ( body_w / 2,           body_l / 2),
        ( body_w / 2,          -body_l / 2),
        (-body_w / 2,          -body_l / 2),
    ]
    body = (
        cq.Workplane("XY")
        .polyline(body_pts).close()
        .extrude(body_h)
        .translate((0, 0, body_standoff))
    )
    try:
        body = body.edges(">Z").chamfer(0.12)
    except Exception:
        pass  # if chamfer fails on this CQ version, leave the top sharp

    # Optional pin-1 dimple — small recess on top of body, near pin 1 corner.
    # Pin 1 is at footprint (X=-4.6875, Y=-1.27) → STEP space (X=-4.6875, Y=+1.27).
    pin1_dimple = (
        cq.Workplane("XY")
        .center(-body_w / 2 + 0.9, body_l / 2 - 0.9)   # ~0.9mm in from pin-1 corner
        .circle(0.30)
        .extrude(0.3)
        .translate((0, 0, body_standoff + body_h - 0.15))
    )
    body = body.cut(pin1_dimple)

    # Four gull-wing leads (vertical drop at body edge + horizontal foot to pad).
    # With body_w = 7.5 mm and pads at ±4.6875 mm, only ~0.94 mm of lead is
    # visible outside the body — short stubby gull-wings, accurate to a wide
    # SOP-4.
    leads = cq.Workplane("XY")
    vert_h = body_standoff + body_h / 2  # vertical reaches body mid-height
    for sx, sy in [(-1, -1), (-1, 1), (1, 1), (1, -1)]:
        vert_x = sx * (body_edge_x + lead_t / 2)
        vert = (
            cq.Workplane("XY")
            .box(lead_t, lead_w, vert_h, centered=(True, True, False))
            .translate((vert_x, sy * pad_y, 0))
        )
        foot_inner_x = sx * body_edge_x
        foot_outer_x = sx * (pad_x + 0.20)
        foot_len = abs(foot_outer_x - foot_inner_x)
        foot_center_x = (foot_inner_x + foot_outer_x) / 2
        foot = (
            cq.Workplane("XY")
            .box(foot_len, lead_w, lead_t, centered=(True, True, False))
            .translate((foot_center_x, sy * pad_y, lead_t / 2))
        )
        leads = leads.union(vert).union(foot)

    return body, leads


def main():
    import os
    os.makedirs("3dmodels", exist_ok=True)

    # IDC Socket — normal (with pins, accurate model)
    body, pins = idc_female_socket_2x05(printable=False)
    assy = cq.Assembly()
    assy.add(body, name="housing", color=cq.Color(0.1, 0.1, 0.1, 1))      # black
    assy.add(pins, name="pins", color=cq.Color(0.83, 0.69, 0.22, 1))      # gold
    output = "3dmodels/IDC-Socket_2x05_P2.54mm_Vertical.step"
    assy.export(output) if hasattr(assy, 'export') else assy.save(output)
    print(f"Generated: {output}")

    # IDC Socket — printable (hollow cavity, no pins, floats over header)
    body_p, _ = idc_female_socket_2x05(printable=True)
    assy_p = cq.Assembly()
    assy_p.add(body_p, name="housing", color=cq.Color(0.1, 0.1, 0.1, 1))
    output_p = "3dmodels/IDC-Socket_2x05_P2.54mm_Vertical_printable.step"
    assy_p.export(output_p) if hasattr(assy_p, 'export') else assy_p.save(output_p)
    print(f"Generated: {output_p}")

    # OLED Module — 0.96" (small board, header on top edge)
    pcb, glass_dark, glass_trans, display, fpc, pins, spacer = oled_sh1106_0_96inch()
    assy2 = cq.Assembly()
    assy2.add(pcb, name="pcb", color=cq.Color(0.0, 0.2, 0.6, 1))               # AZ-Delivery blue PCB
    assy2.add(glass_dark, name="glass_dark", color=cq.Color(0.08, 0.08, 0.08, 1))  # dark glass (display area)
    assy2.add(glass_trans, name="glass_trans", color=cq.Color(0.4, 0.4, 0.35, 0.6)) # transparent glass (FPC visible)
    assy2.add(display, name="display", color=cq.Color(0.01, 0.01, 0.01, 1))     # near-black active screen
    assy2.add(fpc, name="fpc", color=cq.Color(0.6, 0.45, 0.2, 1))              # tan/brown FPC
    assy2.add(pins, name="oled_pins", color=cq.Color(0.83, 0.69, 0.22, 1))     # gold pins
    assy2.add(spacer, name="spacer", color=cq.Color(0.1, 0.1, 0.1, 1))         # black spacer
    output2 = "3dmodels/OLED_SH1106_0.96inch.step"
    assy2.export(output2) if hasattr(assy2, 'export') else assy2.save(output2)
    print(f"Generated: {output2}")

    # OLED Module — 1.3" (large board, header on side, pin order VDD/GND/SCK/SDA)
    pcb_l, glass_dark_l, glass_trans_l, display_l, fpc_l, pins_l, spacer_l = oled_sh1106_1_3inch()
    assy3 = cq.Assembly()
    assy3.add(pcb_l, name="pcb", color=cq.Color(0.0, 0.2, 0.6, 1))
    assy3.add(glass_dark_l, name="glass_dark", color=cq.Color(0.08, 0.08, 0.08, 1))
    assy3.add(glass_trans_l, name="glass_trans", color=cq.Color(0.4, 0.4, 0.35, 0.6))
    assy3.add(display_l, name="display", color=cq.Color(0.01, 0.01, 0.01, 1))
    assy3.add(fpc_l, name="fpc", color=cq.Color(0.6, 0.45, 0.2, 1))
    assy3.add(pins_l, name="oled_pins", color=cq.Color(0.83, 0.69, 0.22, 1))
    assy3.add(spacer_l, name="spacer", color=cq.Color(0.1, 0.1, 0.1, 1))
    output3 = "3dmodels/OLED_SH1106_1.3inch.step"
    assy3.export(output3) if hasattr(assy3, 'export') else assy3.save(output3)
    print(f"Generated: {output3}")

    # SOP-4 long-creepage opto package (PC817 / LTV-817S-B compatible).
    # Used for U1, U2 on cwl-2.0 — KiCad's bundled Package_SO library doesn't
    # ship a STEP for the wide-body variant, only for the 3.8mm one.
    sop4_body, sop4_leads = sop4_long_creepage()
    assy4 = cq.Assembly()
    assy4.add(sop4_body,  name="body",  color=cq.Color(0.05, 0.05, 0.05, 1))  # black plastic
    assy4.add(sop4_leads, name="leads", color=cq.Color(0.80, 0.80, 0.82, 1))  # silver leads
    output4 = "3dmodels/SOP-4_7.5x4.1mm_P2.54mm.step"
    assy4.export(output4) if hasattr(assy4, 'export') else assy4.save(output4)
    print(f"Generated: {output4}")


if __name__ == "__main__":
    main()
