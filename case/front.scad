use <./display_front.scad>
use <./utils.scad>

module stand(h=15.5) {
    translate([0,0,-5.5])
    difference() {
        cylinder(d=10, h=h, $fn=64);
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
    translate([5, 44, 0])
        let(
            vw    = 100,            // vent area width  (x)
            vh    = 20,             // vent area height (y)
            sw    = 3.5,            // slot width
            pitch = 7,              // perpendicular spacing between slots
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
    stand();

