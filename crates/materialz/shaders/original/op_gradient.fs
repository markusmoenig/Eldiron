outName = "Gradient";
outCategory = "Operator";
inTerminal = "0", "NoName", "vec3(0)";

ParameterGroup = "Gradient";

ParamSlider = "startValue", "Start Point", "0.0", "0.01", "1";
ParamSlider = "step1Value", "Step 1", "0.3", "0.01", "3";
ParamSlider = "step2Value", "Step 2", "0.6", "0.01", "5";
ParamSlider = "endValue", "End Point", "1.0", "0.01", "10";

ParamColor = "color1", "Start Color", "float4( 0.0, 0.0, 0.0, 1 )";
ParamColor = "color2", "Step 1 Color", "float4( 0.3, 0.3, 0.3, 1 )";
ParamColor = "color3", "Step 2 Color", "float4( 0.6, 0.6, 0.6, 1 )";
ParamColor = "color4", "End Color", "float4( 1.0, 1.0, 1.0, 1 )";
ParameterGroup = "About";
ParamDescription = "The output is a gradient of the color and step points applied to the input value.";

vec3 color;
float inValue = (inInput1.x + inInput1.y + inInput1.z) / 3.;

color = mix( color1, color2, smoothstep( startValue, step1Value, inValue ) );
color = mix( color, color3, smoothstep( step1Value, step2Value, inValue ) );
color = mix( color, color4, smoothstep( step2Value, endValue, inValue ) );

outColor = color;

