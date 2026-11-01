outName = "Daves Tileable Value Noise";
outCategory = "2D Noise";
ParameterGroup = "Daves Value Noise";
ParamSlider = "scaleValue", "Scale", "10", "0", "40";
ParameterGroup = "About";
ParamDescription = "Fast, tileable 2D value noise.";

// Copyright by Dave Hoskins. Used with permission.
// https://www.shadertoy.com/view/4dlGW2

inInput3 = inInput3.x == 0. ? vec3(scaleValue) : inInput3;

vec2 p = inUV * inInput3.x;
vec2 f;

f = fract(p);
p = floor(p);

f = f*f*(3.0-2.0*f);

vec2 h1 = mod(p, inInput3.x);
float h1f = fract(sin(dot(h1, vec2(27.16898, 38.90563))) * 5151.5473453);

vec2 h2 = mod(p + vec2( 1.0, 0.0), inInput3.x);
float h2f = fract(sin(dot(h2, vec2(27.16898, 38.90563))) * 5151.5473453);

vec2 h3 = mod(p + vec2( 0.0, 1.0), inInput3.x);
float h3f = fract(sin(dot(h3, vec2(27.16898, 38.90563))) * 5151.5473453);

vec2 h4 = mod(p + vec2( 1.0, 1.0), inInput3.x);
float h4f = fract(sin(dot(h4, vec2(27.16898, 38.90563))) * 5151.5473453);

float noise = mix(mix( h1f, h2f,f.x), mix( h3f, h4f, f.x),f.y);

outColor = vec3( noise );
