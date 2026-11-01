outName = "Grass";
outCategory = "Pattern";
inTerminal = "0", "Noise 2D", "vec3(0)";

ParameterGroup = "Grass";
ParamSlider = "scale", "Scale", "1.0", "0.1", "100.0";
ParameterGroup = "About";
ParamHtml = "<html><p>Computes a grass like pattern. Requires a 2D input noise. Inspired by this <a href=https://www.shadertoy.com/view/4slGD4>shader</a>.</p></html>";

// Inspired by this shader from Dave Hoskins. https://www.shadertoy.com/view/4slGD4

vec3 pos = inPos * scale;

float a = InputFunc( inPos, pos.xz*.023, inNormal, inTime, inInput1, inInput2, inInput3 ).x * .5+.15;
float b = InputFunc( inPos, pos.xz*.03, inNormal, inTime, inInput1, inInput2, inInput3 ).x * .6+.25;

vec3 c = vec3(a,b, 0.0);
c *= (inNormal.y - 0.65)*.6;

outColor = c;

