outName = "Random Gradient";
outCategory = "Pattern";
inTerminal = "0", "Noise 2D", "vec3(0)";

ParameterGroup = "Random Gradient";
ParamList = "dir", "Direction", "1", "x, y";
ParamSlider = "seed", "Seed", "1.0", "0.0", "10";
ParamSlider = "globalScale", "Global Scale", "1.0", "0.0", "10";
ParamVector2 = "localScale", "Local Scale", "1.0", "1.0", "-1000", "1000";
ParameterGroup = "About";
ParamDescription = "Generates a random gradient in the specified direction. You can smooth the gradient by increasing the scale.";

vec2 uv = inUV / globalScale * 1./localScale;

float value = dot( vec3( InputFunc( inPos, uv.dir * 5.134 + vec2(2.013) * seed, inNormal, inTime, inInput1, inInput2, inInput3 ).x,
			        InputFunc( inPos, uv.dir * 15.002 + vec2(3.591) * seed, inNormal, inTime, inInput1, inInput2, inInput3 ).x,
					InputFunc( inPos, uv.dir * 25.922 + vec2(4.277) * seed, inNormal, inTime, inInput1, inInput2, inInput3 ).x ),
				   vec3( .85, .35, .17 ) );

outColor = vec3( value );

