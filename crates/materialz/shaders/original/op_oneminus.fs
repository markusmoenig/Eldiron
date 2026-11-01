outName = "One Minus";
outCategory = "Operator";
inTerminal = "0", "NoName", "vec3(0)";

ParameterGroup = "One Minus Operator";
ParamSlider = "value", "Value", "1.0", "0.0", "50";
ParameterGroup = "About";
ParamDescription = "The output is the result of the value property (by default one) minus the input value.";

outColor = value - inInput1;
