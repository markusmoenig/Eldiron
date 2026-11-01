outName = "Sandstone";
outCategory = "Pattern";
inTerminal = "0", "Noise 2D", "vec3(0)";

ParameterGroup = "Sandstone";
ParamSlider = "scale", "Scale", "1.0", "0.1", "100.0";
ParameterGroup = "About";
ParamHtml = "<html><p>Generates a sandstone like pattern. Requires a 2D input noise. Inspired by this <a href=https://www.shadertoy.com/view/4slGD4>shader</a>.</p></html>";

// Inspired by this shader from Dave Hoskins. https://www.shadertoy.com/view/4slGD4

vec3 pos = inPos * scale;

float c = clamp(InputFunc( inPos, pos.xz*.05, inNormal, inTime, inInput1, inInput2, inInput3 ).x, 0.0,1.0);//*10.8;
c += InputFunc( inPos, pos.xz*.1+inNormal.yz*1.08, inNormal, inTime, inInput1, inInput2, inInput3 ).x*.85;
c *= .55;
vec3 color = mix(vec3(.63*c+.2, .7*c+.1, .7*c+.1), vec3(c*.43+.1, c*.3+.2, c*.35+.1), c*.65);
color = color * vec3(c*color.x+.36, c*color.y+.30, c*color.z+.28);

outColor = color;

