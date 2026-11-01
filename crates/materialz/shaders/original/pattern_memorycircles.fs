outName = "Memory Circles";
outCategory = "Pattern";

ParameterGroup = "Memory Circles";
ParamSlider = "scale", "Scale", "1.0", "0.0", "10";
ParameterGroup = "About";
ParamDescription = "Generates a circle based pattern.";

// Inspired by https://www.shadertoy.com/view/4tjyDG

vec2 coord = inUV / scale / 8.;

const float resolution = 1500.0;
const float tileSize = 0.03;
const vec2 tileCenter = vec2(tileSize / 2.0, tileSize / 2.0);

vec2 tile = vec2(floor(coord.x / tileSize), floor(coord.y / tileSize));
coord -= tile * tileSize;
float dist = length(tileCenter - coord);
float circle = cos(dist * resolution / 1.0);
float c = circle;
outColor = vec3( c );
