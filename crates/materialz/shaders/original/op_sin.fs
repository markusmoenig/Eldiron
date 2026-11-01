outName = "Sin";
outCategory = "Operator";
inTerminal = "0", "NoName", "vec3(0)";

ParameterGroup = "Sin Operator";
ParamSlider = "scale", "Scale", "1.0", "0.0", "10";
ParameterGroup = "About";
ParamDescription = "The output is the sine of the input value multiplied by the scale property.";

outColor = sin( inInput1 * scale );
