outName = "Min";
outCategory = "Mixer";
inTerminal = "0", "x", "vec3(1)";
inTerminal = "1", "y", "vec3(1)";
inTerminal = "2", "z", "vec3(1)";

ParameterGroup = "About";
ParamHtml = "The output is the minimum value of the inputs.";

outColor = min( min( inInput1, inInput2), inInput3 );
