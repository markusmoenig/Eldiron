outName = "Rotate 2D";
outCategory = "Mixer";
inTerminal = "0", "x", "vec3(0)";
inTerminal = "1", "angle", "vec3(0.5)";

ParameterGroup = "About";
ParamDescription = "The output is the input x value rotated in 2D space by the angle input.";

vec2 a = sin(vec2(1.5707963, 0) + inInput2.x * 360. );
mat2 r = mat2(a, -a.y, a.x);

outColor = vec3( inInput1.xy * r, 0 );
