outName = "Scratches";
outCategory = "Pattern";

ParameterGroup = "Scratches Shape";
ParamSlider = "SCALE", "Scale", "1.0", "0.0", "10.0";
ParamToolTip = "Scales the shape.";
ParamSlider = "uLayers", "Layers", "4", "1", "20";
ParamToolTip = "The amount of scratch layers.";
ParamSlider = "uWavyness", "Wavyness", "0.1", "0.001", "1.0";
ParamToolTip = "The amount of wavyness / rounding of each scratch.";

ParameterGroup = "About";
ParamDescription = "Generates layers of scratches.";

// --- Based on https://www.shadertoy.com/view/4syXRD

<! FUNC pR !>
void <!pR!>(inout vec2 p, float a)
{
#define <!uScale!> vec2(3.0, 3.0)
#define <!uOffset!> vec2(0.0, 0.0)
#define <!uBaseFrequency!> vec2(0.5, 0.5)
#define <!uFrequencyStep!> vec2(0.25, 0.25)

    float sa = sin(a);
    float ca = cos(a);
    p *= mat2(ca, sa, -sa, ca);
}
<! FUNC END !>

<! FUNC scratch !>
float <!scratch!>(vec2 uv, vec2 seed, float wavyness )
{
    seed.x = floor(sin(seed.x * 51024.0) * 3104.0);
    seed.y = floor(sin(seed.y * 1324.0) * 554.0);

    uv = uv * 2.0 - 1.0;
    <!pR!>(uv, seed.x + seed.y);
    uv += sin(seed.x - seed.y);
    uv = clamp(uv * 0.5 + 0.5, 0.0, 1.0);

    float s1 = sin(seed.x + uv.y * 3.1415) * wavyness;
    float s2 = sin(seed.y + uv.y * 3.1415) * wavyness;

    float x = sign(0.01 - abs(uv.x - 0.5 + s2 + s1));
    return clamp(((1.0 - pow(uv.y, 2.0)) * uv.y) * 2.5 * x, 0.0, 1.0);
}
<! FUNC END !>

<! FUNC layer !>
float <!layer!>(vec2 uv, vec2 frequency, vec2 offset, float angle, float wavyness )
{
    <!pR!>(uv, angle);
    uv = uv * frequency + offset;
    return <!scratch!>(fract(uv), floor(uv), wavyness);
}
<! FUNC END !>

<! FUNC scratches !>
float <!scratches!>(vec2 uv, const int layers, float wavyness )
{
    uv *= <!uScale!>;
    uv += <!uOffset!>;
    vec2 frequency = <!uBaseFrequency!>;
    float scratches = 0.0;
    for(int i = 0; i < layers; ++i)
    {
        float fi = float(i);
    	scratches += <!layer!>(uv, frequency, vec2(fi, fi), fi * 3145.0, wavyness);
        frequency += <!uFrequencyStep!>;
    }
    return scratches;
}
<! FUNC END !>

vec2 uv = inUV / SCALE;

const float AA = 4.;
const int AA2 = int(AA*AA);
float col = 0.0;
vec2 pix = 2.0/inResolution.yy/AA; // or iResolution.xy
for (int i=0; i<AA2; i++){

    float k = float(i);
    vec2 uvOffs = uv + vec2(floor(k/AA), mod(k, AA)) * pix;
    col += <!scratches!>(uvOffs, uLayers, uWavyness);
}
col /= (AA*AA);

outColor = vec3(col);
