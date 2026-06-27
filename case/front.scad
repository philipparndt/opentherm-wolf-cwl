use <./display_front.scad>
use <./utils.scad>

module stand(h=15.5, o=10) {
    translate([0,0,-5.5])
    difference() {
        cylinder(d=o, h=h, $fn=64);
        cylinder(d=4.2, h=h, $fn=64);
    }
}

module stands() {
    translate([-91.2/2, 0, 0])
        for (i = [0:1]) {
            for (j = [0:0]) {
                translate([i*91.8, j*42, 0])
                    stand();
            }
        }

    *translate([-91.2/2, 80, -1.6]) {
        stand(h=15.5+1.6);
        translate([91.8, 0, 0])
            stand(h=15.5+1.6);
    }
}

offset=3;
border=2;
case_width = 110;
case_length = 72;
case_radius = 10;
h=20;

// Wall-mount ear: 2mm thick tab sticking out from the case side,
// flush with the bottom plane. long=true gives an adjustment slot.
module mounting_latch(long=false) {
    ear_w   = 12;   // width along the wall (y)
    hole_x  = 6.5;  // hole centre distance from the wall (x)
    tail    = 4;    // material beyond the hole centre (x)
    ear_l   = hole_x + tail;   // protrusion beyond the wall (x)
    overlap = border;   // reach into the wall only, not the interior cavity
    t       = 2;    // thickness (z)
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

module case_front() {
    // Main case front
    control_x=34+1.5;
    control_y=21;
    display_x=case_width/2+18.5+1.5;
    display_y = 5.1;

    difference() {
        union() {
            translate([0,0,-10])
            difference() {
                linear_extrude(height=h+offset)
                    rounded_rectangle(width=case_width, length=case_length, r=case_radius, fn=64);

                translate([border, border, -border])
                    linear_extrude(height=h)
                        rounded_rectangle(width=case_width-border*2, length=case_length-border*2, r=case_radius-border, fn=64);

            }

            translate([control_x,control_y,1])
                cylinder(d=27, h=9, $fn=64);

            // Mounting ears (left: round hole, right: adjustment slot)
            translate([0, case_length/2 - 6, -10])
                mirror([1,0,0])
                    mounting_latch(long=false);

            translate([case_width, case_length/2 - 6, -10])
                mounting_latch(long=true);
        }

        translate([display_x, display_y, 10-4])
            display_cutout();

        translate([control_x,control_y,-50])
            cylinder(d=24, h=100, $fn=64);

    }


    difference() {
        translate([control_x,control_y,1])
            cylinder(d=27, h=2, $fn=64);

        translate([control_x,control_y,1])
            cylinder(d=10, h=2, $fn=64);

    }


    translate([display_x, display_y, 10])
        rotate([0,180,0])
            display();

    translate([case_width/2,9,0])
        stands();

}

difference() {
    case_front();

    translate([0,0,10])
        cube([150,39,20]);

    translate([border,40,-8])
        cube([case_width-border*2,20,20]);

    translate([border, 51, -border])
        linear_extrude(height=10+offset+1)
            rounded_rectangle(width=case_width-border*2, length=19, r=case_radius-border, fn=64);

    // Ethernet cutout
    translate([0,41+3+6,-8-3])
        cube([10,25-2.7-6,20-5.5+3]);

    // Kabel
    translate([-5,41+5,0])
        rotate([0,90,0])
            cylinder(d=4.8, h=10, $fn=128);

    // "Lüftung" - diagonal 45° slots covering the 100x20 area
    translate([5, 45, 0])
        let(
            vw    = 100,            // vent area width  (x)
            vh    = 19,             // vent area height (y)
            sw    = 3,              // slot width
            pitch = 6,              // perpendicular spacing between slots
            step  = pitch * sqrt(2),
            cb    = (vh - vw) / 2   // centre b (= y - x) of the full-length band
        )
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

translate([case_width-8.3, case_length-6.6, 0])
    stand(h=15.5+2, o=8);

