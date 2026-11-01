outName = "Masked Color Blend";
outCategory = "Mixer";
inTerminal = "0", "Pattern", "vec3(0)";
inTerminal = "1", "Mask", "vec3(1)";

ParameterGroup = "Masked Color Blend - Color Set 1";
ParamColor = "color1", "Color 1", "float4(1.0, 1.0, 1.0, 1.0)";
ParamColor = "color2", "Color 2", "float4(0.0, 0.0, 0.0, 1.0)";
ParamSlider = "power1", "Power", "1.0", "0.001", "10";
ParameterGroup = "Color Set 2";
ParamColor = "color3", "Color 1", "float4(0.0, 0.0, 0.0, 1.0)";
ParamColor = "color4", "Color 2", "float4(1.0, 1.0, 1.0, 1.0)";
ParamSlider = "power2", "Power", "1.0", "0.001", "10";

ParameterGroup = "About";
ParamDescription = "The output is the interpolation between two colors by the pattern input and optionally raises the pattern by the power property. The mask input switches between two differnt parameters sets.";

outColor = mix( mix( color1, color2, pow(inInput1.x, power1 )), mix( color3, color4, pow(inInput1.x, power2 )), inInput2 );
