outName = "Domain Warping";
outCategory = "2D fBm";
inTerminal = "0", "Noise", "vec3(0)";

ParameterGroup = "Turbulence";
ParamSlider = "layers", "Layers", "5", "2", "20";
ParameterGroup = "About";
ParamDescription = 'Domain Warping.';

// --- Inspired by https://www.shadertoy.com/view/llySRV

vec2 st = inUV / 3.;
vec2 _st = st;

// -- q.x

float v = 0.0;
float a = 0.5;
vec2 shift = vec2(100.0);
mat2 rot = mat2(cos(0.5), sin(0.5), -sin(0.5), cos(0.50));
vec2 q = vec2(0.);

for (int i = 0; i < layers; i++)
{
    v += a * InputFunc( inPos, _st, inNormal, inTime, inInput1, inInput2, inInput3 ).x;
    _st = rot * _st * 2.0 + shift;
    a *= 0.5;
}

q.x = v;

// -- q.y

_st = st + vec2(1.0);

v = 0.0;
a = 0.5;

for (int i = 0; i < layers; i++)
{
    v += a * InputFunc( inPos, _st, inNormal, inTime, inInput1, inInput2, inInput3 ).x;
    _st = rot * _st * 2.0 + shift;
    a *= 0.5;
}

q.y = v;

// -- r.x

vec2 r = vec2(0.);

_st = st + 1.0*q + vec2(1.7,9.2)+ 0.15*inTime;

v = 0.0;
a = 0.5;

for (int i = 0; i < layers; i++)
{
    v += a * InputFunc( inPos, _st, inNormal, inTime, inInput1, inInput2, inInput3 ).x;
    _st = rot * _st * 2.0 + shift;
    a *= 0.5;
}

r.x = v;

// -- r.y

_st = st + 1.0*q + vec2(8.3,2.8)+ 0.126*inTime;

v = 0.0;
a = 0.5;

for (int i = 0; i < layers; i++)
{
    v += a * InputFunc( inPos, _st, inNormal, inTime, inInput1, inInput2, inInput3 ).x;
    _st = rot * _st * 2.0 + shift;
    a *= 0.5;
}

r.y = v;

// -- Final

_st = st+r;
v = 0.0;
a = 0.5;

for (int i = 0; i < layers; i++)
{
    v += a * InputFunc( inPos, _st, inNormal, inTime, inInput1, inInput2, inInput3 ).x;
    _st = rot * _st * 2.0 + shift;
    a *= 0.5;
}

outColor = vec3( v );

