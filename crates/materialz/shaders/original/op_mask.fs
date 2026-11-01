outName = "Mask";
outCategory = "Operator";
inTerminal = "0", "NoName", "vec3(0)";
ParameterGroup = "About";
ParamDescription = "The output is 0.0 when the input value is 0.0 as well, 1.0 otherwise.";

outColor = clamp( abs( inInput1 * 999999999999.0 ), 0., 1. );
