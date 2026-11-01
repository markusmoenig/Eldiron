outName = "Noise Hole Mask";
outCategory = "2D Pattern";
inTerminal = "0", "Noise 2D", "vec3(0)";
inDefaultTerminal = "Brians Value Noise";

ParameterGroup = "Noise Hole Mask Pattern";
ParamSlider = "globalScale", "Global Scale", "1.0", "0.0", "10";
ParamVector2 = "localScale", "Local Scale", "1.0", "1.0", "-1000", "1000";
ParamSlider = "offset", "Offset", "3.", "0.1", "10";
ParamToolTip = "Defines the shape of the mask.";
ParamSlider = "cutOff", "Cut Off", "0.6", "0.0", "1";
ParamToolTip = "The smaller the cut off, the larger the area covered by the mask.";
ParameterGroup = "About";
ParamDescription = "Generates a noise based pattern of solid shapes / holes which can be used as a mask. Requires a 2D input noise.";

vec2 p = inUV / globalScale * 1./localScale;

float noise = InputFunc( inPos, p, inNormal, inTime, inInput1, inInput2, inInput3 ).x;
noise += InputFunc( inPos, p * offset, inNormal, inTime, inInput1, inInput2, inInput3 ).x * 0.2;

vec3 col = vec3( 0 );
if ( noise < cutOff ) col = vec3( 1 );

outColor = col;

