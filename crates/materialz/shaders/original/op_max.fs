outName = "Max";
outCategory = "Operator";
inTerminal = "0", "NoName", "vec3(0)";

ParameterGroup = "Max Operator";
ParamSlider = "globalValue", "Global", "0.0", "0.0", "10";
ParamVector3 = "localValue", "Local", "1.0", "1.0", "1.0", "-1000", "1000";
ParameterGroup = "About";
ParamDescription = "The output is the greater of the input value and the global property mulitplied with the local property.";

outColor = max( inInput1, (globalValue * localValue) );
