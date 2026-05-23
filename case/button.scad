// Handle opener for Linea Mini R knobs

// Parameters
diameter = 22;          // Main body diameter
height = 15;            // Extrusion height
grip_count = 12;        // Number of grip cutouts
grip_depth = 1;         // How deep the grip cutouts go
grip_cylinder_d = 8;    // Diameter of the cylinders for rounded cutouts

fillet_radius = 4;      // Top edge fillet radius

engrave_depth = 0.6;    // Depth of engraved text and arrow
arrow_radius = 10;      // Radius of the curved arrow
arrow_width = 2;        // Width of arrow line

inner_radius = 3.05;      // Inner radius for the knob tab cutout
outer_radius = 3.5;       // Outer radius for the knob tab cutout

$fn = 128;

difference() {
    // Main cylinder body
    cylinder(d = diameter, h = height);

    // Grip cutouts around the perimeter
    for (i = [0:grip_count-1]) {
        angle = i * 360 / grip_count;
        rotate([0, 0, angle])
            translate([diameter/2 + grip_cylinder_d/2 - grip_depth, 0, -1])
                cylinder(d = grip_cylinder_d, h = height + 2);
    }

    translate([0,0,height/2-1])
        cylinder(r=inner_radius, h=height-2, center=true);

    translate([0,0,2/2])
        cylinder(r=outer_radius , h=2, center=true);


    // Top edge fillet
    translate([0, 0, height])
        rotate_extrude()
            translate([diameter/2 - fillet_radius, -fillet_radius])
                difference() {
                    square([fillet_radius + 1, fillet_radius]);
                    circle(r = fillet_radius);
                }
}

translate([1.4/2-inner_radius,0,height-8])
cube([1.4, 10, 10], center=true);
