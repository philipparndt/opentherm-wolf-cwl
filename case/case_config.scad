// Shared case configuration. Included by front.scad and back.scad so both
// parts stay dimensionally in sync.

offset      = 3;
border      = 2;
case_width  = 110;
case_length = 72;
case_radius = 10;
h           = 20;

standoff_chamfers = true;   // lead-in chamfers on the standoff screw holes
slim              = true;   // pull bottom + left + right walls in to the round standoffs
slim_inset        = 0.9;    // clearance between each round standoff edge and the inner wall

// Mounting-ear thickness (z) per part.
latch_h_front = 2;          // front shell ears
latch_h_back  = 1;          // back plate ears

back_lip_w    = 1.2;        // back plate locating-lip wall thickness

// Round standoff centres (PCB-fixed), as placed by stands().
rstand_o  = 10;
rstand_xL = case_width/2 - 45.6;   // 9.4
rstand_xR = case_width/2 + 46.2;   // 101.2
rstand_y  = 9;

// Case outer boundary. slim brings bottom/left/right in to the standoffs; top fixed.
x0 = slim ? rstand_xL - rstand_o/2 - slim_inset - border : 0;
x1 = slim ? rstand_xR + rstand_o/2 + slim_inset + border : case_width;
y0 = slim ? rstand_y  - rstand_o/2 - slim_inset - border : 0;
y1 = case_length;
cw = x1 - x0;        // outer width
cl = y1 - y0;        // outer length

bottom_z   = -10;    // z of the case floor / wall bottom
stand_sink = 5.5;    // how far a standoff body drops below z = 0
