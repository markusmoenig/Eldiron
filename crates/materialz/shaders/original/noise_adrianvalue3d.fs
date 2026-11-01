outName = "Adrians Value Noise";
outCategory = "3D Noise";

ParameterGroup = "Adrians Value Noise";
ParamSlider = "globalScale", "Global Scale", "1.0", "0.0", "10";
ParamVector3 = "localScale", "Local Scale", "1.0", "1.0", "1.0", "-1000", "1000";
ParameterGroup = "About";
ParamDescription = "Fast 3D value noise.";

// Provided by Adrian Chlubek and based on https://www.shadertoy.com/view/MllBRr, used with permission.

vec3 x = inPos / globalScale * 1. / localScale * 10.;
vec3 p = floor(x);
vec3 fr = fract(x);
vec3 LBZ = p + vec3(0.0, 0.0, 0.0);
vec3 LTZ = p + vec3(0.0, 1.0, 0.0);
vec3 RBZ = p + vec3(1.0, 0.0, 0.0);
vec3 RTZ = p + vec3(1.0, 1.0, 0.0);

vec3 LBF = p + vec3(0.0, 0.0, 1.0);
vec3 LTF = p + vec3(0.0, 1.0, 1.0);
vec3 RBF = p + vec3(1.0, 0.0, 1.0);
vec3 RTF = p + vec3(1.0, 1.0, 1.0);

float l0candidate1 = fract(4768.1232345456 * sin((LBZ.x+LBZ.y*43.0+LBZ.z*137.0)));
float l0candidate2 = fract(4768.1232345456 * sin((RBZ.x+RBZ.y*43.0+RBZ.z*137.0)));
float l0candidate3 = fract(4768.1232345456 * sin((LTZ.x+LTZ.y*43.0+LTZ.z*137.0)));
float l0candidate4 = fract(4768.1232345456 * sin((RTZ.x+RTZ.y*43.0+RTZ.z*137.0)));

float l0candidate5 = fract(4768.1232345456 * sin((LBF.x+LBF.y*43.0+LBF.z*137.0)));
float l0candidate6 = fract(4768.1232345456 * sin((RBF.x+RBF.y*43.0+RBF.z*137.0)));
float l0candidate7 = fract(4768.1232345456 * sin((LTF.x+LTF.y*43.0+LTF.z*137.0)));
float l0candidate8 = fract(4768.1232345456 * sin((RTF.x+RTF.y*43.0+RTF.z*137.0)));

float l1candidate1 = mix(l0candidate1, l0candidate2, fr[0]);
float l1candidate2 = mix(l0candidate3, l0candidate4, fr[0]);
float l1candidate3 = mix(l0candidate5, l0candidate6, fr[0]);
float l1candidate4 = mix(l0candidate7, l0candidate8, fr[0]);


float l2candidate1 = mix(l1candidate1, l1candidate2, fr[1]);
float l2candidate2 = mix(l1candidate3, l1candidate4, fr[1]);


float l3candidate1 = mix(l2candidate1, l2candidate2, fr[2]);

outColor = vec3( l3candidate1 );
