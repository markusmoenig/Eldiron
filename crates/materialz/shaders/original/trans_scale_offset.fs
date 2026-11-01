outName = "Transform";
outCategory = "Transform";
inTerminal = "0", "Shape", "vec3(0)";
inDefaultTerminal = "Disc";

ParameterGroup = "Warp";
ParamSlider = "globalScale", "Global", "1.0", "0.0", "10.0";
ParamToolTip = "Global scale of the shape. If shape is used inside a pattern the scale should be 1. as the pattern will scale the shape automatically.";
ParamVector2 = "localScale", "Local", "1", "1", "0", "10";
ParamToolTip = "Local X/Y scale of the shape. If shape is used inside a pattern the scale should be 1. as the pattern will scale the shape automatically.";
ParamVector2 = "offset", "Offset", "0", "0", "-10", "10";
ParamSlider = "rot", "Rotate", "0.0", "0.0", "360";
ParameterGroup = "About";
ParamDescription = "Scales, rotates and moves the input shape.";

vec2 U = (inUV-offset) * 1./globalScale * 1./localScale;

vec2 a = sin(vec2(1.5707963, 0) + radians(rot) );
mat2 r = mat2(a, -a.y, a.x);

U = U * r;

outColor = InputFunc( inPos, U, inNormal, inTime, inInput1, inInput2, inInput3 );

