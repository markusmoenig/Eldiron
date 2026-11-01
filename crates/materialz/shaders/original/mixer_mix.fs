outName = "Mix";
outCategory = "Mixer";
inTerminal = "0", "x", "vec3(0)";
inTerminal = "1", "y", "vec3(1)";
inTerminal = "2", "a", "vec3(0.5)";

ParameterGroup = "About";
ParamDescription = "The output is a linear interpolation between x and y using a to weight between them.";

outColor = mix( inInput1, inInput2, inInput3 );
