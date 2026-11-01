outName = "Multiply";
outCategory = "Operator";
inTerminal = "0", "NoName", "vec3(0)";

ParameterGroup = "Multiply Operator";
ParamSlider = "globalValue", "Global", "1.0", "0.0", "10";
ParamVector3 = "localValue", "Local", "1.0", "1.0", "1.0", "-100000", "100000";
ParameterGroup = "About";
ParamDescription = "The output is the result of the multiplication of the input value with the global and local properties.";

outColor = inInput1 * globalValue * localValue;
