outName = "Average";
outCategory = "Mixer";
inTerminal = "0", "x", "vec3(0)";
inTerminal = "1", "y", "vec3(0)";
inTerminal = "2", "z", "vec3(0)";

ParameterGroup = "About";
ParamDescription = "The output is the average of the inputs.";

outColor = (inInput1 + inInput2 + inInput3) / 3.0;

