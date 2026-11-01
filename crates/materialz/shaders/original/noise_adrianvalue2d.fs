outName = "Adrians Value Noise";
outCategory = "2D Noise";

ParameterGroup = "Adrians Value Noise";
ParamSlider = "globalScale", "Global Scale", "1.0", "0.0", "10";
ParamVector2 = "localScale", "Local Scale", "1.0", "1.0", "-1000", "1000";
ParameterGroup = "About";
ParamDescription = "Fast 2D value noise.";

// Provided by Adrian Chlubek and based on https://www.shadertoy.com/view/MllBRr, used with permission.

vec2 x = inUV / globalScale * 1. / localScale * 10.;
vec2 p = floor(x);
vec2 fr = fract(x);
vec2 LB = p;
vec2 LT = p + vec2(0.0, 1.0);
vec2 RB = p + vec2(1.0, 0.0);
vec2 RT = p + vec2(1.0, 1.0);

float LBo = fract(4768.1232345456 * sin((LB.x+LB.y*43.0)));
float RBo = fract(4768.1232345456 * sin((RB.x+RB.y*43.0)));
float LTo = fract(4768.1232345456 * sin((LT.x+LT.y*43.0)));
float RTo = fract(4768.1232345456 * sin((RT.x+RT.y*43.0)));

float noise1d1 = mix(LBo, RBo, fr.x);
float noise1d2 = mix(LTo, RTo, fr.x);

float noise2d = mix(noise1d1, noise1d2, fr.y);

outColor = vec3( noise2d );
