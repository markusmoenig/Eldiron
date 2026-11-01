outName = "Wood";
outCategory = "3D Pattern";
inTerminal = "0", "Noise 3D", "vec3(0)";
inDefaultTerminal = "Brians Perlin Noise";

ParameterGroup = "Wood Pattern";
ParamSlider = "globalScale", "Global Scale", "1.0", "0.0", "10";
ParamVector3 = "localScale", "Local Scale", "1.0", "1.0", "1.0", "-1000", "1000";
ParamColor = "color1", "Wood Color", "float4(0.78, 0.549, 0.235, 1.0)";
ParamColor = "color2", "Ring Color", "float4(0.38, 0.212, 0.059, 1.0)";
ParameterGroup = "About";
ParamDescription = "Generates a wood ring pattern. Requires a 3D input noise.";

// Inspired by a shader from Otaviogood. https://www.shadertoy.com/view/ldscDM

vec3 pos = inPos / globalScale * 1./localScale * 1./40.;

float a = InputFunc( pos*vec3(8.0, 1.5, 8.0), inUV, inNormal, inTime, inInput1, inInput2, inInput3 ).x;
float b = InputFunc( pos*vec3(-pos*vec3(8.0, 1.5, 8.0)+4.5678), inUV, inNormal, inTime, inInput1, inInput2, inInput3 ).x;

float rings = length( pos.xz + vec2( a, b * 0.05) * 64.0);
rings = pow(sin(rings)*0.5+0.5, 8.0) + cos(rings)*0.7 + 0.7;
rings /= 1.8;

vec3 color = mix(color1, color2, rings);
color = max(vec3(0.0), color);

float c = InputFunc( pos*64.0*vec3(1.0, 0.2, 1.0), inUV, inNormal, inTime, inInput1, inInput2, inInput3 ).x;

float rough = c * 0.1 + 0.9;
color *= rough;

outColor = clamp( color, 0., 1. );

