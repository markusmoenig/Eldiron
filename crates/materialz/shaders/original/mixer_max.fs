outName = "Max";
outCategory = "Mixer";
inTerminal = "0", "x", "vec3(0)";
inTerminal = "1", "y", "vec3(0)";
inTerminal = "2", "z", "vec3(0)";

ParameterGroup = "About";
ParamDescription = "The output is the maximum value of the inputs.";

outColor = max( max( inInput1, inInput2), inInput3 );
