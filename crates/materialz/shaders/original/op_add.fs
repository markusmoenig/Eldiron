outName = "Add";
outCategory = "Operator";
inTerminal = "0", "NoName", "vec3(0)";

ParameterGroup = "Add Operator";
ParamSlider = "globalValue", "Global", "0.0", "0.0", "10";
ParamVector3 = "localValue", "Local", "0.0", "0.0", "0.0", "-1000", "1000";
ParameterGroup = "About";
ParamDescription = "The output is the input value added to the global and local properties.";

outColor = inInput1 + globalValue + localValue;
