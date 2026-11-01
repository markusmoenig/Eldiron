outName = "Split";
outCategory = "Operator";
inTerminal = "0", "NoName", "vec3(0)";

ParameterGroup = "Split Operator";
ParamList = "component", "Component", "0", "x, y, z";
ParameterGroup = "About";
ParamDescription = "The output is one of the x, y or z components of the input value.";

outColor = vec3( inInput1.component );
