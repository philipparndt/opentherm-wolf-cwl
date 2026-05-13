$fn=200;


d=30.5;
dn=2.8;

difference () {

    cube([35.6, 33.8, 1.5]);

    translate([2.5,2,0]) {
        cylinder(h=5, d=dn, center=true);
        translate([d,0,0])
            cylinder(h=5, d=dn, center=true);

        translate([0,28.8,0]) {
            cylinder(h=5, d=dn, center=true);
            translate([d,0,0])
                cylinder(h=5, d=dn, center=true);
        }    
    }
}