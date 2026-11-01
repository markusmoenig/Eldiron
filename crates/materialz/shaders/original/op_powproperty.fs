outName = "Power of Property";
outCategory = "Operator";
inTerminal = "0", "Value", "vec3(0)";

ParameterGroup = "Power of Property Operator";
ParamSlider = "power", "Power", "1.0", "0.0", "100";
ParameterGroup = "About";
ParamDescription = "The output is the input value raised by the power of the power property.";

outColor = pow( inInput1, vec3( power ) );
