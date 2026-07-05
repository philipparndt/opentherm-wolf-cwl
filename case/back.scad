use <./utils.scad>
use <./front.scad>          // mounting_latch
include <./case_config.scad>

// ============================ Back plate ============================
// Flat plate closing the open back of the case. A small step nests inside the
// front's cavity to locate it, and it carries the same mounting ears as the
// front. Modelled flat-face-down (z=0) for printing.

plate_t       = 1;     // flat plate thickness
step_h        = 3;   // locating lip height (sits inside the front cavity)
fit_clearance = 0.5;   // gap between the lip and the inner cavity wall

module back_plate() {
    inset = border + fit_clearance;   // lip distance from the outer edge
    lip_w = back_lip_w;               // lip wall thickness (a thin locating line)

    // closed flat plate on the case outer footprint
    linear_extrude(height=plate_t)
        rounded_rectangle(width=cw, length=cl, r=case_radius, fn=64);

    difference() {
        // thin locating lip stepping up into the cavity
        translate([0, 0, plate_t])
            linear_extrude(height=step_h)
                difference() {
                    translate([inset, inset])
                        rounded_rectangle(width=cw-2*inset, length=cl-2*inset,
                        r=case_radius-inset, fn=64);
                    translate([inset+lip_w, inset+lip_w])
                        rounded_rectangle(width=cw-2*(inset+lip_w), length=cl-2*(inset+lip_w),
                        r=case_radius-inset-lip_w, fn=64);
                }

        translate([0, 0, plate_t])
            cube([20,20,10]);

        translate([cw-20, 0, plate_t])
            cube([20,20,10]);

        translate([0, cl-20, 0]) {
            translate([0, 0, plate_t])
                cube([20,20,10]);

            translate([cw-20, 0, plate_t])
                cube([20,20,10]);
        }
    }
}

back_plate();

// Mounting ears (same as the front): left round hole, right adjustment slot,
// flush with the flat outer face.
translate([0,  cl/2 - 6, 0])
    mirror([1,0,0])
        mounting_latch(long=false, t=latch_h_back);

translate([cw, cl/2 - 6, 0])
    mounting_latch(long=true, t=latch_h_back);
