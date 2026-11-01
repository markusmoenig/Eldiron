outName = "Fizzers Tiling";
outCategory = "Pattern";

ParameterGroup = "Fizzers Tiling Pattern";
ParamSlider = "border_size", "Border Size", "0.015", "0.0", "0.5";
ParamSlider = "scale", "Scale", "1.0", "0.0", "10";
ParameterGroup = "About";
ParamDescription = "Generates square tiles with rotated, square corners.";

// Based on https://www.shadertoy.com/view/lsj3zy

vec2 p = inUV / scale * 2.;

float corner_size = border_size;

vec2 c0 = floor(p);
vec2 c1 = (p-c0)-vec2(0.5);
vec2 rc1 = (c1.x*vec2(1.0)+c1.y*vec2(1.0,-1.0))*0.6;

float br = max(smoothstep(0.5-border_size,0.5,abs(c1.y)),smoothstep(0.5-border_size,0.5,abs(c1.x)));
float cr = max(smoothstep(0.5-corner_size,0.5,abs(rc1.y)),smoothstep(0.5-corner_size,0.5,abs(rc1.x)));

outColor = vec3(1.0-max(cr,br));

