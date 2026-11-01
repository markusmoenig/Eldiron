outName = "Square Root";
outCategory = "Operator";
inTerminal = "0", "NoName", "vec3(0)";

ParameterGroup = "Square Root Operator";
ParamSlider = "scale", "Scale", "1.0", "0.0", "10";
ParameterGroup = "About";
ParamDescription = "The output is the square root of the input value.";

outColor = sqrt( inInput1 * scale );
