outName = "Terrain fBm";
outCategory = "2D fBm";
inTerminal = "0", "Noise", "vec3(0)";

ParameterGroup = "Terrain fBm";
ParamSlider = "layers", "Layers", "5", "2", "20";
ParameterGroup = "About";
ParamHtml = "Generates a rough terrain.";

// Based on https://www.shadertoy.com/view/lslfRN used with permission.

vec2 p = inPos.xz;

p /= 8.;
p += .5;

float a = 1., sum = 0., res = 0.;

for (int i=0; i< layers; i++)
{
    res += InputFunc( inPos, p, inNormal, inTime, inInput1, inInput2, inInput3 ).x * a;
    p = mat2(1, -.75, .75, 1)*p*2.72;
    sum += a;
    a *= -.5/1.7;
}
outColor = vec3( res/sum );
