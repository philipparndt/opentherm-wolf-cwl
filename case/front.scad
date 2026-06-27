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

module case_front() {
    // Main case front
    control_x=34+1.5;
    control_y=21;
    display_x=case_width/2+18.5+1.5;
    display_y = 5.1;

    difference() {
        union() {
            difference() {
                linear_extrude(height=10+offset)
                    rounded_rectangle(width=case_width, length=case_length, r=case_radius, fn=64);

                translate([border, border, -border])
                    linear_extrude(height=10)
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
    translate([0,41+3+6,-8])
        cube([10,25-2.7-6,20-5.5]);

    // "Lüftung"
    translate([5,41+8,0])
        cube([100,10,20]);
}

translate([case_width-8.3, case_length-6.6, 0])
    stand();


