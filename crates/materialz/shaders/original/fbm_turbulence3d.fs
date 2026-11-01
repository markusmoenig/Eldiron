outName = "Turbulence";
outCategory = "3D fBm";
inTerminal = "0", "Noise", "vec3(0)";

ParameterGroup = "Turbulence";
ParamSlider = "layers", "Layers", "5", "1", "10";
ParamSlider = "lacunarity", "Lacunarity", "2.0", "0.001", "10";
ParamSlider = "gain", "Gain", "0.5", "0.001", "3";
ParameterGroup = "Scale";
ParamSlider = "globalScale", "Global", "1.0", "0.0", "10";
ParamVector3 = "localScale", "Local", "1.0", "1.0", "1.0", "-1000", "1000";
ParameterGroup = "About";
ParamDescription = "Generates a layered turbulence / fbm.";

inPos = inPos / globalScale * 1./localScale;

float res = 0.;
float amp = 1.;
float tot = 0.;

for(int i=0; i < layers; i++){
    res += InputFunc( inPos, inUV, inNormal, inTime, inInput1, inInput2, inInput3 ).x * amp;
    inUV * lacunarity;
    inPos *= lacunarity;
    tot += amp;
    amp *= gain;

}

outColor = vec3( res/tot );
