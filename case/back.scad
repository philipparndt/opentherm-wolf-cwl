use <./display_front.scad>
use <./utils.scad>

module stand(h=15.5) {
    translate([0,0,-5.5])
    difference() {
        cylinder(d=10, h=h, $fn=64);
        cylinder(d=4.2, h=h, $fn=64);
    }
}

module stand_hole(h=15.5) {
    translate([0,0,21.5])
        difference() {
            *cylinder(d=4.2, h=h, $fn=64);
            cylinder(d1=4.2, d2=8, h=3, $fn=64);
        }
}

module stands() {
    translate([-91.2/2, 0, 0])
        for (i = [0:1]) {
            for (j = [0:1]) {
                translate([i*91.8, j*42, 0])
                    stand();
            }
        }

    translate([-91.2/2, 80, 0]) {
        stand(h=15.5);
        translate([91.8, 0, 0])
            stand(h=15.5);
    }
}

module stands_holes() {
    translate([-91.2/2, 0, 0])
        for (i = [0:1]) {
            for (j = [0:1]) {
                translate([i*91.8, j*42, 0])
                    stand_hole();
            }
        }

    translate([-91.2/2, 80, 0]) {
        stand_hole(h=15.5);
        translate([91.8, 0, 0])
            stand_hole(h=15.5);
    }
}


module case_back() {
    case_width = 110;
    case_length = 97;
    case_radius = 10;

    border=2;

    control_x=23;
    control_y=34-4.8;
    display_x=16-4.2;

    difference() {
        union() {
            difference() {
                union() {
                    difference() {
                        linear_extrude(height=22.5+border)
                            rounded_rectangle(width=case_width, length=case_length, r=case_radius, fn=64);

                        translate([border, border, -border])
                            linear_extrude(height=22.5+border)
                                rounded_rectangle(width=case_width-border*2, length=case_length-border*2, r=case_radius-border, fn=64);

                    }
                }
            }

            translate([case_width/2,9,13])
                stands();
        }

        translate([case_width/2,9,0])
            stands_holes();

        // ethernet cutout
        translate([case_width/2-5,case_length-5,-.5])
            cube([17, 30, 18]);

        // power
        translate([10,case_length-25,-.5])
            rotate([0,0,90])
                cube([10, 30, 18]);


    }



}

case_back();

