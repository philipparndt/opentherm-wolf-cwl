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
            for (j = [0:1]) {
                translate([i*91.8, j*42, 0])
                    stand();
            }
        }

    translate([-91.2/2, 80, -1.6]) {
        stand(h=15.5+1.6);
        translate([91.8, 0, 0])
            stand(h=15.5+1.6);
    }
}

module case_front() {
    // Main case front
    case_width = 110;
    case_length = 97;
    case_radius = 10;

    border=2;

    control_x=23;
    control_y=34-4.8;
    display_x=16-4.2;

    difference() {
        *linear_extrude(height=10)
            rounded_rectangle(width=case_width, length=case_length, r=case_radius, fn=64);

        union() {
            difference() {
                linear_extrude(height=10)
                    rounded_rectangle(width=case_width, length=case_length, r=case_radius, fn=64);

                translate([border, border, -border])
                    linear_extrude(height=10)
                        rounded_rectangle(width=case_width-border*2, length=case_length-border*2, r=case_radius-border, fn=64);

            }

            translate([control_x,control_y,1])
                cylinder(d=27, h=9, $fn=64);
        }

        translate([case_width/2, display_x, 10-4])
            display_cutout();

        translate([control_x,control_y,-50])
            cylinder(d=24, h=100, $fn=64);

        //    translate([0,60,0])
        //        cube([150,100,20]);

    }

    difference() {
        translate([control_x,control_y,1])
            cylinder(d=27, h=2, $fn=64);

        translate([control_x,control_y,1])
            cylinder(d=10, h=2, $fn=64);

    }


    translate([case_width/2, display_x, 10])
        rotate([0,180,0])
            display();

    translate([case_width/2,9,0])
        stands();

}

case_front();