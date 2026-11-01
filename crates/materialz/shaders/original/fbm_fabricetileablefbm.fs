outName = "Fabrices Tileable Perlin fBM";
outCategory = "2D fBm";
inTerminal = "0", "Noise", "vec3(0)";
inDefaultTerminal = "Fabrices Perlin Noise";

ParameterGroup = "Fabrices Tileable Perlin fBm";
ParamSlider = "scale", "Scale", "1.0", "0.0", "10.0";
ParamToolTip = "The scale of the noise.";
ParamVector2 = "CYCLE", "Cycle", "0.5", "0.5", "0", "1";
ParamToolTip = "The repetition cycle of the noise.";
ParamSlider = "layers", "Layers", "3", "1", "20";
ParamToolTip = "The layers of turbulence.";
ParameterGroup = "About";
ParamDescription = "Tileable 2D Perlin turbulence. Needs Fabrices Tileable Perlin Noise as input.";

<! FUNC fbmT2 !>
float <!fbmT2!>(vec2 p, vec2 cycle, float scale, const int layers) {
    float v = 0.,  a = .5;

    for (int i = 0; i < layers; i++, p*=2.,a/=2.,cycle*=2.)
        v += a * InputFunc( vec3(0), p, vec3(0), 0., vec3(0), vec3(0), vec3(scale, cycle) ).x;
    return v;
}
<! FUNC END !>

vec2 U = inUV;

float noise = <!fbmT2!>(U,CYCLE*scale, scale, layers);
outColor = vec3( noise );

