use <./display_front.scad>
use <./utils.scad>

// ============================ Parameters ============================
// Shared parameters and derived boundary live in case_config.scad so the back
// plate stays dimensionally in sync.
include <./case_config.scad>

// ============================ Helper modules ============================

// Screw bore: cored hole for post-print tapping, with optional lead-in
// chamfers at both ends. Cut from z = zbot to z = ztop.
module screw_bore(bore, zbot, ztop, ch=0.8) {
    eps = 0.1;
    translate([0,0,zbot-eps])
        cylinder(d=bore, h=ztop-zbot+2*eps, $fn=64);
    if (standoff_chamfers) {
        translate([0,0,zbot-eps])
            cylinder(d1=bore+2*ch, d2=bore, h=ch+eps, $fn=64);
        translate([0,0,ztop-ch])
            cylinder(d1=bore, d2=bore+2*ch, h=ch+eps, $fn=64);
    }
}

// Plain standoff. For SLM 316L the bore is a cored hole tapped after printing
// (bore = tap-drill diameter: M3->2.5, M4->3.3, M5->4.2).
module stand(h=15.5, o=10, bore=4.2) {
    difference() {
        translate([0,0,-stand_sink])
            cylinder(d=o, h=h, $fn=64);
        screw_bore(bore, -stand_sink, h-stand_sink);
    }
}

// Solid (un-hollowed) outer body of the case. Also used to trim standoff ribs
// so they conform to the rounded wall instead of poking through it.
module case_outer_solid() {
    translate([x0, y0, bottom_z])
        linear_extrude(height=h+offset)
            rounded_rectangle(width=cw, length=cl, r=case_radius, fn=64);
}

// Standoff braced into a corner: a (smaller) boss with long-hole ribs toward
// the two near walls. sx,sy = brace direction signs (-1/+1 in x and y). The
// ribs overshoot, then trim to the case outer surface, and the screw hole is
// re-cut where the ribs filled over it.
module braced_stand(px, py, sx, sy, o=8, hh=15.5, bore=4.2) {
    bw    = o;          // rib width matches the boss
    reach = 22;         // rib overshoot; trimmed back to the wall
    ax    = sx > 0 ? 0 : 180;
    ay    = sy > 0 ? 90 : -90;

    translate([px, py, 0])
    difference() {
        union() {
            stand(h=hh, o=o, bore=bore);
            intersection() {
                translate([0,0,-stand_sink])
                    linear_extrude(height=hh) {
                        rotate([0,0,ax]) translate([-bw/2,-bw/2]) long_hole(reach, bw);
                        rotate([0,0,ay]) translate([-bw/2,-bw/2]) long_hole(reach, bw);
                    }
                translate([-px,-py,0])
                    case_outer_solid();
            }
        }
        screw_bore(bore, -stand_sink, hh-stand_sink);
    }
}

// The two round, PCB-mounting standoffs. In slim mode they shrink and brace
// into their side+bottom corners; otherwise they are plain bosses.
module stands() {
    if (slim) {
        braced_stand(rstand_xL, rstand_y, -1, -1, o=8);
        braced_stand(rstand_xR, rstand_y, +1, -1, o=8);
    } else {
        translate([rstand_xL, rstand_y, 0]) stand();
        translate([rstand_xR, rstand_y, 0]) stand();
    }
}

// Wall-mount ear: 2mm thick tab sticking out from the case side, flush with the
// bottom plane. long=true gives an adjustment slot instead of a round hole.
module mounting_latch(long=false, t=2) {
    ear_w   = 12;   // width along the wall (y)
    hole_x  = 6.5;  // hole centre distance from the wall (x)
    tail    = 4;    // material beyond the hole centre (x)
    ear_l   = hole_x + tail;
    overlap = border;   // reach into the wall only, not the interior cavity
    r       = 3;    // corner radius
    hole_d  = 4;
    slot    = 4;    // extra travel for the long hole

    difference() {
        linear_extrude(height=t)
            translate([-overlap, 0])
                rounded_rectangle(width=ear_l+overlap, length=ear_w, r=r, fn=64);

        translate([hole_x, ear_w/2, -0.5])
            if (long)
                rotate([0,0,90])
                    linear_extrude(height=t+1)
                        translate([-(hole_d+slot)/2, -hole_d/2])
                            long_hole(hole_d+slot, hole_d);
            else
                cylinder(d=hole_d, h=t+1, $fn=64);
    }
}

// Diagonal 45° vent slots filling a (vw x vh) area anchored at the origin.
module vent(vw=100, vh=19, sw=3, pitch=6) {
    step = pitch * sqrt(2);
    cb   = (vh - vw) / 2;   // centre b (= y - x) of the full-length band
    for (i = [-9 : 9])
        let(
            b   = cb + i * step,
            xlo = max(0, -b),
            xhi = min(vw, vh - b),
            dx  = xhi - xlo,
            cx  = (xlo + xhi) / 2,
            cy  = cx + b,
            len = dx * sqrt(2) - sw   // inset rounded caps so they stay inside
        )
        if (len > 2)
            translate([cx, cy, 0])
                rotate([0, 0, 45])
                    linear_extrude(height = vh)
                        translate([-len/2, -sw/2])
                            long_hole(len, sw);
}

// ============================ Case ============================

module case_front() {
    control_x = 34 + 1.5;
    control_y = 21;
    display_x = case_width/2 + 18.5 + 1.5;
    display_y = 5.1;

    difference() {
        union() {
            // hollow shell
            difference() {
                case_outer_solid();
                translate([x0+border, y0+border, bottom_z-border])
                    linear_extrude(height=h)
                        rounded_rectangle(width=cw-2*border, length=cl-2*border, r=case_radius-border, fn=64);
            }

            // control-knob boss
            translate([control_x, control_y, 1])
                cylinder(d=27, h=9, $fn=64);

            // Mounting ears (left: round hole, right: adjustment slot)
            translate([x0, (y0+y1)/2 - 6, bottom_z])
                mirror([1,0,0])
                    mounting_latch(long=false, t=latch_h_front);
            translate([x1, (y0+y1)/2 - 6, bottom_z])
                mounting_latch(long=true, t=latch_h_front);
        }

        translate([display_x, display_y, 10-4])
            display_cutout();

        translate([control_x, control_y, -50])
            cylinder(d=24, h=100, $fn=64);
    }

    // control-knob trim ring
    difference() {
        translate([control_x, control_y, 1]) cylinder(d=27, h=2, $fn=64);
        translate([control_x, control_y, 1]) cylinder(d=10, h=2, $fn=64);
    }

    translate([display_x, display_y, 10])
        rotate([0,180,0])
            display();

    stands();
}

// ============================ Build ============================

difference() {
    case_front();

    // open the front over the lower section
    translate([0,0,10])
        cube([150,39,20]);

    // top inner opening (square + rounded slot)
    translate([x0+border, 40, -8])
        cube([cw-2*border, 20, 20]);
    translate([x0+border, 51, -border])
        linear_extrude(height=10+offset+1)
            rounded_rectangle(width=cw-2*border, length=19, r=case_radius-border, fn=64);

    // Ethernet cutout (left wall)
    translate([x0, 50, -11])
        cube([10, 17.5, 17.5]);

    // Cable pass-through (left wall)
    translate([x0-5, 46, 0])
        rotate([0,90,0])
            cylinder(d=4.8, h=10, $fn=128);

    // Ventilation slots (top wall)
    translate([5, 45, 0])
        vent();
}

// top-right corner standoff, braced toward the right (+x) and top (+y) walls
braced_stand(x1-8.3, y1-6.6, +1, +1, o=8, hh=15.5+2);
