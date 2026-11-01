outName = "Power of Input";
outCategory = "Operator";
inTerminal = "0", "Power", "vec3(0)";

ParameterGroup = "Power of Input Operator";
ParamSlider = "value", "Value", "1.0", "0.0", "100";
ParameterGroup = "About";
ParamDescription = "The output is the value property raised by the power of the input value.";

outColor = pow( vec3( value ), inInput1 );
