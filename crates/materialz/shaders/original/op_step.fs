outName = "Step";
outCategory = "Operator";
inTerminal = "0", "NoName", "vec3(0)";

ParameterGroup = "Step Operator";
ParamSlider = "globalValue", "Global Edge", "0.0", "0.0", "10";
ParamVector3 = "localValue", "Local Edge", "0.0", "0.0", "0.0", "-1000", "1000";
ParameterGroup = "About";
ParamDescription = "The output is the 0.0 if the input value is smaller than the sum of the global and local edge properties, 1.0 otherwise.";

outColor = step( globalValue + localValue, inInput1 );
