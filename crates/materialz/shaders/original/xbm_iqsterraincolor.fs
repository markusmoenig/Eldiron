outName = "IQs Terrain Coloring";
outCategory = "2D fBm";
inTerminal = "0", "Noise", "vec3(0)";

ParameterGroup = "Turbulence";
ParamSlider = "scale", "Scale", "1.0", "0.1", "100.0";
ParamColor = "higherRockColor", "Higher Rock", "float4( 0.08 * 10,0.05 * 10,0.03 * 10, 1 )";
ParamColor = "lowerRockColor", "Lower Rock", "float4( 0.45,.30,0.15, 1 )";
ParamColor = "grassColor", "Grass", "float4( 0.3, 0.3, 0.1, 1 )";
ParameterGroup = "About";
ParamHtml = "<html><p>Computes a layered turbulence / fbm on the values of the input layer.</p></html>";

vec3 pos = inPos * scale;
float SC = 1.;

// rock
float r = InputFunc( inPos,  (7.0)/SC*pos.xz/256., inNormal, inTime, inInput1, inInput2, inInput3 ).x;
vec3 color = (r*0.25+0.75)*0.9*mix( higherRockColor / 10., vec3(0.10,0.09,0.08),  InputFunc( inPos,  0.00007*vec2(pos.x,pos.y*48.0)/SC, inNormal, inTime, inInput1, inInput2, inInput3 ).x );
color = mix( color, 0.20*lowerRockColor*(0.50+0.50*r),smoothstep(0.70,0.9,inNormal.y) );
color = mix( color, 0.15*grassColor*(0.25+0.75*r),smoothstep(0.95,1.0,inNormal.y) );

float h = smoothstep(55.0,80.0,pos.y/SC + 25.0*InputFunc( inPos, 0.01*pos.xz/SC, inNormal, inTime, inInput1, inInput2, inInput3 ).x );
float e = smoothstep(1.0-0.5*h,1.0-0.1*h,inNormal.y);
float o = 0.3 + 0.7*smoothstep(0.0,0.1,inNormal.x+h*h);
float s = h*e*o;
color = mix( color, 0.29*vec3(0.62,0.65,0.7), smoothstep( 0.1, 0.9, s ) );

outColor = vec3( color );
