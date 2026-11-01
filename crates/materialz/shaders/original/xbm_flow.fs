outName = "Flow";
outCategory = "2D fBm";
inTerminal = "0", "Noise", "vec3(0)";
ParameterGroup = "Flow";
ParamSlider = "scale", "Scale", "0.05", "0.001", "20";
ParamSlider = "rot", "Rotation", "0.0", "0.0", "360";
ParameterGroup = "About";
ParamDescription = "Creates a flow like turbulence.";

vec2 a = sin(vec2(1.5707963, 0) + rot);
mat2 m = mat2(a, -a.y, a.x);

vec2 p = scale * (inUV);
vec2 uv = p;

for( int i=0; i<50; i++ )
    uv += 0.00015 * m * (-1.0+2.0*InputFunc(inPos, 0.5*uv, inNormal, inTime, inInput1, inInput2, inInput3 ).xz);

float f = length( uv - p )*95.0;

outColor = vec3( f );
