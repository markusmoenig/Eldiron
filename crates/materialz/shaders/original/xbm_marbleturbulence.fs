outName = "Marble Turbulence";
outCategory = "3D fBm";
inTerminal = "0", "Noise 3D", "vec3(0)";

ParameterGroup = "About";
ParamHtml = "<html><p>Simulates a marble texture. Needs a 3D noise as input.</p></html>";

#define Noise_Marble(g) InputFunc( g, inUV, inNormal, inTime, inInput1, inInput2, inInput3 )

vec3 p = inPos;
// p.yz = p.yz*vec2(.5,sqrt(3.0)*.5)+p.zy*vec2(sqrt(3.0)*.5,-.5);
p *= 32.0*vec3(1,2,1);
float f = abs(Noise_Marble(p)*2.0-1.0).x; p *= .5;
f = mix( f, abs(Noise_Marble(p)*2.0-1.0).x, .5 ); p *= .5;
f = mix( f, abs(Noise_Marble(p)*2.0-1.0).x, .5 ); p *= .5;
f = mix( f, abs(Noise_Marble(p)*2.0-1.0).x, .5 ); p *= .5;
f = mix( f, abs(Noise_Marble(p)*2.0-1.0).x, .5 ); p *= .5;

outColor = mix( vec3(1), vec3(0), pow(1.0-f,12.0) );
