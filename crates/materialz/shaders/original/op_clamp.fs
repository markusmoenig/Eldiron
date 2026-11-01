outName = "Clamp";
outCategory = "Operator";
inTerminal = "0", "NoName", "vec3(0)";

ParameterGroup = "Clamp Operator";
ParamVector3 = "value1", "Low", "0.0", "0.0", "0.0", "-1000", "1000";
ParamVector3 = "value2", "High", "1.0", "1.0", "1.0", "-1000", "1000";
ParameterGroup = "About";
ParamDescription = "The output is the input value constrained to lie between the low and high properties.";

outColor = clamp( inInput1, value1, value2 );
