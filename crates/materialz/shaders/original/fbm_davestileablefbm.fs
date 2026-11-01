outName = "Daves Tileable Value fBM";
outCategory = "2D fBm";
inTerminal = "0", "Noise", "vec3(0)";
inDefaultTerminal = "Daves Value Noise";

ParameterGroup = "Daves Tileable Value fBM";
ParamSlider = "layers", "Layers", "5", "2", "20";
ParamSlider = "amplitude", "Amplitude", "0.6", "0.001", "10";
ParamSlider = "gain", "Gain", "0.5", "0.001", "3";
ParamSlider = "scaleValue", "Scale", "10", "0", "40";
ParameterGroup = "About";
ParamDescription = "Tileable 2D turbulence. Needs Daves Tileable Value Noise as input.";

// Copyright by Dave Hoskins. Used with permission.
// https://www.shadertoy.com/view/4dlGW2.

float scale = float(scaleValue);

float f = 0.0;
vec2 p = mod(inUV, scale);
float amp = amplitude;

for (int i = 0; i < layers; i++)
{
    f += InputFunc( inPos, p, inNormal, inTime, inInput1, inInput2, vec3(scale) ).x * amp;
    amp *= gain;
    scale *= 2.;
}

outColor = vec3( min(f, 1.0) );

