outName = "Rotate 2D";
outCategory = "Operator";
inTerminal = "0", "NoName", "vec3(0)";

ParameterGroup = "Rotate 2D Property";
ParamSlider = "rot", "Angle", "0.0", "0.0", "360";
ParameterGroup = "About";
ParamDescription = "The output is the input value 2D rotated by the angle property.";

vec2 a = sin(vec2(1.5707963, 0) + rot);
mat2 r = mat2(a, -a.y, a.x);

outColor = vec3( inInput1.xy * r, 0 );
