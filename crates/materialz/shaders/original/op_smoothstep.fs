outName = "Smooth Step";
outCategory = "Operator";
inTerminal = "0", "NoName", "vec3(0)";

ParameterGroup = "Smooth Step Operator";
ParamVector3 = "lowerEdge", "Lower Edge", "0.0", "0.0", "0.0", "-1000", "1000";
ParamVector3 = "upperEdge", "Upper Edge", "1.0", "1.0", "1.0", "-1000", "1000";
ParameterGroup = "About";
ParamDescription = "The output is a smooth hermite interpolation between 0..1 if the input value is located between the lower and upper edge. 0.0 or 1.0 otherwise.";

outColor = smoothstep( lowerEdge, upperEdge, inInput1 );
