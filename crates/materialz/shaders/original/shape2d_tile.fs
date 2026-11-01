outName = "Tile";
outCategory = "Shape";

ParameterGroup = "Tile Shape";
ParamSlider = "ROUND", "Rounding", "0.0", "0.0", "1";
ParamToolTip = "Edge rounding of the tiles.";
ParamSlider = "inBEVEL", "Bevel", "0.01", "0.0", "1";
ParamToolTip = "Beveling.";
ParamList = "PROFILE", "Profile", "0", "Linear, SSlope, TopRounded, BottomRounded";
ParameterGroup = "Gradient", "false";
ParamBoolean = "GRAD", "Random Gradient", "0";
ParamToolTip = "Gives the shape a random gradient, useful for generating uneveness through visually tilting the shape.";
ParamSlider = "inStrengthGradient", "Gradient Strength", "0.5", "0.0", "1";
ParamToolTip = "Strength of the gradient.";
ParameterGroup = "Size & Offset", "false";
ParamSlider = "globalScale", "Global", "1.0", "0.0", "1.0";
ParamToolTip = "Global scale of the shape. If shape is used inside a pattern the scale should be 1. as the pattern will scale the shape automatically.";
ParamVector2 = "localScale", "Local", "1", "1", "0", "1";
ParamToolTip = "Local X/Y scale of the shape. If shape is used inside a pattern the scale should be 1. as the pattern will scale the shape automatically.";
ParamVector2 = "offset", "Offset", "0", "0", "0", "100";
ParamToolTip = "X/Y offset of the shape. If shape is used inside a pattern the offset should be 0. as the pattern will position the shape automatically.";
ParameterGroup = "About";
ParamDescription = "Generates a tile shape. This is the default shape for brick and all other rectangular shaped patterns.";

const int Linear = 0, SSlope = 1, TopRounded = 2, BottomRounded = 3;

vec2  BEVEL = vec2(inBEVEL);

vec2  grad = vec2(1,0);
float grad_randA = 2.,
      grad_randD = 360.;

vec2 U = inUV / globalScale * localScale - offset;
vec2 Box = inInput1.xy == vec2( 0 ) ? vec2( 1, 1 ) : inInput1.xy;
float id = inInput1.z;
vec3 O = vec3( 0 );

vec2 A = Box/2. - abs(U), B  = A * 2. / BEVEL;

float m = min(B.x,B.y);

if (A.x<ROUND && A.y<ROUND)
      m = (ROUND-length(ROUND-A)) *2./dot(BEVEL,normalize(ROUND-A));

m = clamp( m, 0., 1. );
if ( PROFILE == 0 )
      O += m;
else if ( PROFILE == 1 )
      O += smoothstep(0.,1.,m);
else if ( PROFILE == 2 )
      O += sqrt(1.-(1.-m)*(1.-m));
else if ( PROFILE == 3 )
      O += 1. - sqrt(1.-m*m);

if ( GRAD ) {
      uvec3 x = uvec3(id,3,1);
      const uint k = 1103515245U;
      x = ((x>>8U)^x.yzx)*k;
      x = ((x>>8U)^x.yzx)*k;
      x = ((x>>8U)^x.yzx)*k;
      vec3 rotHash = (vec3(x)*(1.0/float(0xffffffffU)));

      vec3 R = -1.+2.*rotHash;
      mat2 rot = mat2(sin(radians(grad_randD/2.*R.x) + vec4(1,0,2,1)*1.5707));

      grad *= rot * pow(grad_randA,R.y);
      O.rgb *= 3. - inStrengthGradient * 3. + dot(grad,U);
}

outColor = O;
