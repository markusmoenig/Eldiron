outName = "Checker B/W";
outCategory = "Pattern";

ParameterGroup = "Checker Pattern";
ParamSlider = "scale", "Scale", "5.0", "0.0", "20";
ParameterGroup = "About";
ParamDescription = "Generates a simple b/w checkerboard pattern.";

vec2 uv = floor(scale * inUV.xy);
outColor = vec3(mod(uv.x + uv.y, 2.));


