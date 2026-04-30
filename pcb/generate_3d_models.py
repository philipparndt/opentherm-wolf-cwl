"""
Generate 3D STEP models for custom PCB components.

Run with: python generate_3d_models.py
Output: 3dmodels/IDC-Socket_2x05_P2.54mm_Vertical.step
"""

import cadquery as cq


def idc_female_socket_2x05():
    """
    2x5 IDC female socket, 2.54mm pitch.
    Body: 17.1 x 5.8 x 9.5mm
    Keying bump: 3.7mm wide, 1.3mm standout, 6mm tall from top
    Chamfer: 1.2mm wide, 2.8mm tall on top edges
    Rectangular pin holes
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

    # Main body
    body = (
        cq.Workplane("XY")
        .box(body_l, body_d, body_h, centered=(True, True, False))
        .translate((cx, cy, 0))
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

    # Rectangular pin holes through the body
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

    # Apply rotation (90° Z) and offset (x=2.5, y=-10.15, z=0.25)
    # to align with the KiCad footprint pads
    import math
    angle = math.radians(90)

    def transform(shape):
        return shape.rotateAboutCenter((0, 0, 1), 90).translate((2.5 - 6.3, -10.15 + 3.8, 0.25))

    body = transform(body)
    pin_bodies = transform(pin_bodies)

    return body, pin_bodies


def oled_sh1106_1_3inch():
    """
    1.3" SH1106 128x64 OLED module with 4-pin I2C header and mounting holes.
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


def main():
    import os
    os.makedirs("3dmodels", exist_ok=True)

    # IDC Socket
    body, pins = idc_female_socket_2x05()
    assy = cq.Assembly()
    assy.add(body, name="housing", color=cq.Color(0.1, 0.1, 0.1, 1))      # black
    assy.add(pins, name="pins", color=cq.Color(0.83, 0.69, 0.22, 1))      # gold
    output = "3dmodels/IDC-Socket_2x05_P2.54mm_Vertical.step"
    assy.export(output) if hasattr(assy, 'export') else assy.save(output)
    print(f"Generated: {output}")

    # OLED Module
    pcb, glass_dark, glass_trans, display, fpc, pins, spacer = oled_sh1106_1_3inch()
    assy2 = cq.Assembly()
    assy2.add(pcb, name="pcb", color=cq.Color(0.0, 0.2, 0.6, 1))               # AZ-Delivery blue PCB
    assy2.add(glass_dark, name="glass_dark", color=cq.Color(0.08, 0.08, 0.08, 1))  # dark glass (display area)
    assy2.add(glass_trans, name="glass_trans", color=cq.Color(0.4, 0.4, 0.35, 0.6)) # transparent glass (FPC visible)
    assy2.add(display, name="display", color=cq.Color(0.01, 0.01, 0.01, 1))     # near-black active screen
    assy2.add(fpc, name="fpc", color=cq.Color(0.6, 0.45, 0.2, 1))              # tan/brown FPC
    assy2.add(pins, name="oled_pins", color=cq.Color(0.83, 0.69, 0.22, 1))     # gold pins
    assy2.add(spacer, name="spacer", color=cq.Color(0.1, 0.1, 0.1, 1))         # black spacer
    output2 = "3dmodels/OLED_SH1106_1.3inch.step"
    assy2.export(output2) if hasattr(assy2, 'export') else assy2.save(output2)
    print(f"Generated: {output2}")


if __name__ == "__main__":
    main()
