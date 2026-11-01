outName = "White Noise (Hash)";
outCategory = "2D Noise";
ParameterGroup = "About";
ParamDescription = 'Hash based white noise.';

float noise = fract(sin(dot(inUV.xy, vec2(12.9898,78.233))) * 43758.5453123);
outColor = vec3( noise );
