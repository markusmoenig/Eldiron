outName = "Multiply";
outCategory = "Mixer";
inTerminal = "0", "In 1", "vec3(1)";
inTerminal = "1", "In 2", "vec3(1)";
inTerminal = "2", "In 3", "vec3(1)";

ParameterGroup = "About";
ParamDescription = "The output is the result of the multiplication of the inputs.";

outColor = inInput1 * inInput2 * inInput3;

