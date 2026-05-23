module display_cutout() {
    translate([-38/2,0,0])
    difference() {
        cube([38, 35, 4]);
    }
}

module display() {
    translate([-38/2,0,0]) {
        union() {
            difference() {
                cube([38, 35, 4]);

                // display cutout
                translate([(38 - 34.7) / 2, 6.3, 0])
                    cube([34.7, 19, 5]);

                translate([(38 - 36) / 2, (35 - 34) / 2, 1])
                    cube([36, 34, 5]);
            }

            steg_d = 1;
            translate([0, 6.3 - steg_d, 0])
                cube([38, steg_d, 2]);

            translate([6.3, 35 - 4, 0])
                cube([3.5, 4, 1.8]);

            translate([38 - 6.3 - 3.5, 35 - 4, 0])
                cube([3.5, 4, 1.8]);
        }
    }
}

display();
