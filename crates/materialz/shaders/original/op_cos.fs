outName = "Cos";
outCategory = "Operator";
inTerminal = "0", "NoName", "vec3(0)";

ParameterGroup = "Cos Operator";
ParamSlider = "scale", "Scale", "1.0", "0.0", "10";
ParameterGroup = "About";
ParamDescription = "The output is the cosine of the input value multiplied by the scale property.";

outColor = cos( inInput1 * scale );
