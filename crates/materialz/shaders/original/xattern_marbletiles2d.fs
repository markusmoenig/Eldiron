outName = "Marble Tiles";
outCategory = "Pattern";
inTerminal = "0", "Noise 3D", "vec3(0)";

ParameterGroup = "Marble Tiles 3D Pattern";
ParamSlider = "globalScale", "Global Scale", "1.0", "0.0", "10";
ParamVector3 = "localScale", "Local Scale", "1.0", "1.0", "1.0", "-1000", "1000";
ParameterGroup = "About";
ParamDescription = "Generates a tiled marble pattern. The returned x component will hold the marble pattern, the y component the tiling mask and the z component the proposed depth. Requires a 3D input noise.";

// Inspired by this shader from TekF: https://www.shadertoy.com/view/MdXSzX

vec3 pos = inPos / (globalScale * localScale / 20.0);
vec3 axis;

float tile = mod( floor(5. / globalScale * inUV.x) + floor(5./ globalScale * inUV.y), 2.0);

if ( tile > .0 ) axis = normalize(vec3(1,-3,2));
else axis = normalize(vec3(1,2,3));

vec3 mfp = (pos + dot(pos,axis)*axis*2.0)*2.0;
float marble = 0.0;

marble += abs(InputFunc( mfp, inUV, inNormal, inTime, inInput1, inInput2, inInput3 ).x-.5);
marble += abs(InputFunc( mfp*2., inUV, inNormal, inTime, inInput1, inInput2, inInput3 ).x-.5)/2.0;
marble += abs(InputFunc( mfp*4., inUV, inNormal, inTime, inInput1, inInput2, inInput3 ).x-.5)/4.0;
marble += abs(InputFunc( mfp*8., inUV, inNormal, inTime, inInput1, inInput2, inInput3 ).x-.5)/4.0;

marble /= 1.0-1.0/8.0;
marble = pow( 1.0-marble,10.0);

float depth = (1.-(.95+.05*sin((1.0-marble)*4.*6.283*2.))) * 0.001;

outColor = vec3( marble, tile, depth );

