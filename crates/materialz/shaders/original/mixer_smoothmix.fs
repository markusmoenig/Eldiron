outName = "Smooth Mix";
outCategory = "Mixer";
inTerminal = "0", "x", "vec3(0)";
inTerminal = "1", "y", "vec3(1)";
inTerminal = "2", "a", "vec3(0.5)";

ParameterGroup = "Smooth Mix";
ParamSlider = "smoothFrom", "Smooth A", "0.0", "0.0", "1.0";
ParamSlider = "smoothTo", "Smooth B", "1.0", "0.0", "1.0";
ParameterGroup = "About";
ParamDescription = "The output is the result of a linear interpolation between x and y using a to weight between them. Additionally a is interpolated between the smooth a / b properties using smoothstep.";

outColor = mix( inInput1, inInput2, smoothstep( smoothFrom, smoothTo, inInput3 ) );

