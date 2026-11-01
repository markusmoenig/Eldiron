outName = "Offset";
outCategory = "2D fBm";
inTerminal = "0", "Noise", "vec3(0)";
ParameterGroup = "Offset";
ParamSlider = "offsetIntensity", "Offset", "5.0", "0.01", "20";
ParameterGroup = "About";
ParamDescription = "Offsets the input 2D noise with a noise based offset.";

float o = InputFunc( inPos, inUV + (InputFunc( inPos, inUV, inNormal, inTime, inInput1, inInput2, inInput3 ).x * offsetIntensity), inNormal, inTime, inInput1, inInput2, inInput3 ).x;
outColor = vec3( o );
