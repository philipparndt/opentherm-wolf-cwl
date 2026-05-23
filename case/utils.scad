// Rounded rectangle module
module rounded_rectangle(width, length, r, fn=64) {
    // ensure radius doesn't exceed half of width/height
    rr = min(r, width/2, length/2);

    translate([rr, rr])
        offset(r=rr, $fn=fn)
            square([width - 2*rr, length - 2*rr]);

}