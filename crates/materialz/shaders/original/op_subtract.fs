outName = "Subtract";
outCategory = "Operator";
inTerminal = "0", "NoName", "vec3(0)";

ParameterGroup = "Subtract Operator";
ParamSlider = "globalValue", "Global", "0.0", "0.0", "10";
ParamVector3 = "localValue", "Local", "0.0", "0.0", "0.0", "-1000", "1000";
ParameterGroup = "About";
ParamDescription = "The output is the result of the subtraction of the sum of the local and global properties from the input value.";

outColor = inInput1 - (globalValue + localValue);
