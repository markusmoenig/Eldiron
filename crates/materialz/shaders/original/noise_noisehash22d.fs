outName = "White Noise Scalable (Hash)";
outCategory = "2D Noise";
ParameterGroup = "White Noise";
ParamSlider = "scale", "Scale", "1.0", "0.0", "10";
ParameterGroup = "About";
ParamDescription = 'Scalable Hash based white noise.';

vec2 uv = inUV;
uv = floor(uv*40./scale+80.*vec2(sin(inTime/2.),cos(inTime/2.)));

float hash1 = fract(sqrt(abs(.213*uv.x+.5*uv.y))*3734.421891);
float hash2 = fract(sqrt(abs(.73*uv.y+7.))*3734.421891);

float x = fract(sqrt(abs((fract( hash1 + hash2 )))*3734.421891));

outColor = vec3(x);

