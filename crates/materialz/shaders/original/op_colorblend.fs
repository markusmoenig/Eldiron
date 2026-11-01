outName = "Color Blend";
outCategory = "Operator";
inTerminal = "0", "NoName", "vec3(0)";

ParameterGroup = "Color Blend Operator";
ParamColor = "color1", "Color 1", "float4( 0.0, 0.0, 0.0, 1 )";
ParamColor = "color2", "Color 2", "float4( 1.0, 1.0, 1.0, 1 )";
ParameterGroup = "About";
ParamDescription = "The output is the linear interpolation between the two color properties and the input value.";

outColor = mix( color1, color2, inInput1 );
