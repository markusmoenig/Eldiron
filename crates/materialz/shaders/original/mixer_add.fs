outName = "Add";
outCategory = "Mixer";
inTerminal = "0", "In 1", "vec3(0)";
inTerminal = "1", "In 2", "vec3(0)";
inTerminal = "2", "In 3", "vec3(0)";

ParameterGroup = "About";
ParamDescription = "The output is the result of adding the inputs.";

outColor = inInput1 + inInput2 + inInput3;

