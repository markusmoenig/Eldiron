outName = "Brick";
outCategory = "Pattern";

ParameterGroup = "Brick Pattern";
ParamSlider = "BRICKWIDTH", "Brick Width", "0.25", "0.0", "1";
ParamSlider = "BRICKHEIGHT", "Brick Height", "0.08", "0.0", "1";
ParamSlider = "MORTARTHICKNESS", "Mortar Thickness", "0.01", "0.0", "1";
ParamSlider = "uneveness", "Uneveness", "0.5", "0.0", "1";
ParameterGroup = "About";
ParamDescription = "Generates a brick pattern. Returns the id of the brick in the x component and the depth map in the y component. Use the Split Operator to access the individual components.";

const float BMWIDTH = BRICKWIDTH+MORTARTHICKNESS;
const float BMHEIGHT = BRICKHEIGHT+MORTARTHICKNESS;

const float MWF = MORTARTHICKNESS*0.5/BMWIDTH;
const float MHF = MORTARTHICKNESS*0.5/BMHEIGHT;

float ss = inUV.x / BMWIDTH;
float tt = inUV.y / BMHEIGHT;

if ( mod( tt * 0.5, 1.0 ) > 0.5 )
	ss = ss + 0.5;

float sbrick = floor( ss );
float tbrick = floor( tt );

vec2 randv2 = fract(cos(( float2( (tbrick + 0.5 ), (sbrick + 0.5) ) * vec2(1000.0,1000.0))));
float noise = fract(sin(dot(randv2.xy ,vec2(12.9898,78.233))) * 43758.5453);

// --- Distance size is random, currently disabled

ss = ss - sbrick;
tt = tt - tbrick;
float w = step( MWF, ss ) - step( 1.0 - MWF, ss );
float h = step( MHF, tt ) - step( 1.0 - MHF, tt );

// --- Random color per tile, currently uses fast cos based noise which creates visible patterns

// float color = mix( 1.0, /*float3( 0, 0, 0 )*/ 1.0 / 3.0, noise );

// ---

float sbump = smoothstep(0.0,MWF,ss) - smoothstep(1.0-MWF,1.0,ss);
float tbump = smoothstep(0.0,MHF,tt) - smoothstep(1.0-MHF,1.0,tt);
float bump = sbump * tbump;

float unevenBump = bump;

if ( noise <= 0.25 ) unevenBump -= ss * uneveness;
else if ( noise <= 0.50 ) unevenBump -= tt * uneveness;
else if ( noise <= 0.75 ) unevenBump -= (BMWIDTH-ss) * uneveness;
else unevenBump -= (BMHEIGHT-tt) * uneveness;

outColor = vec3( noise, clamp( unevenBump, 0., 1. ), 0. );

