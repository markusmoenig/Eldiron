outName = "Turbulence";
outCategory = "2D fBm";
inTerminal = "0", "Noise", "vec3(0)";

ParameterGroup = "Turbulence";
ParamSlider = "layers", "Layers", "5", "1", "20";
ParamSlider = "lacunarity", "Lacunarity", "2.0", "0.001", "10";
ParamSlider = "gain", "Gain", "0.5", "0.001", "3";
ParameterGroup = "Scale";
ParamSlider = "globalScale", "Global", "1.0", "0.0", "10";
ParamVector2 = "localScale", "Local", "1.0", "1.0", "-1000", "1000";
ParameterGroup = "About";
ParamDescription = "Generates a layered turbulence / fbm.";

float value = 0.0;
float amplitude = 1.;

vec2 p = inUV / globalScale * 1./localScale;

for (int i = 0; i < layers; i++)
{
    value += amplitude * InputFunc( inPos, p, inNormal, inTime, inInput1, inInput2, inInput3 ).x;
    p *= lacunarity;
    amplitude *= gain;
}

outColor = vec3( value );

