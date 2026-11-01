outName = "Granite";
outCategory = "Pattern";
inTerminal = "0", "Noise 2D", "vec3(0)";

ParameterGroup = "Granite";
ParamSlider = "scale", "Scale", "1.0", "0.1", "100.0";
ParameterGroup = "About";
ParamHtml = "<html><p>Computes a granite like pattern. Requires a 2D input noise. Inspired by this <a href=https://www.shadertoy.com/view/4slGD4>shader</a>.</p></html>";

// Inspired by this shader from Dave Hoskins. https://www.shadertoy.com/view/4slGD4

vec3 pos = inPos * scale;

float c = InputFunc( inPos, vec2(pos.x*.09, pos.z*.095+pos.yy*0.15), inNormal, inTime, inInput1, inInput2, inInput3 ).x;
c += InputFunc( inPos, vec2(pos.x*2.233, pos.z*2.23), inNormal, inTime, inInput1, inInput2, inInput3 ).x*0.5;

outColor = vec3( c * .4 );

