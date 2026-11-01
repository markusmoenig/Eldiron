outName = "Offset";
outCategory = "3D fBm";
inTerminal = "0", "Noise", "vec3(0)";
ParameterGroup = "Offset";
ParamSlider = "offsetIntensity", "Offset", "5.0", "0.01", "20";
ParameterGroup = "About";
ParamDescription = "Offsets the input 3D noise with a noise based offset.";

#define <!Noise!>(g) InputFunc( g, inUV, inNormal, inTime, inInput1, inInput2, inInput3 )

float o = <!Noise!>( inPos + (<!Noise!>( inPos ) * offsetIntensity) ).x;
outColor = vec3( o );

