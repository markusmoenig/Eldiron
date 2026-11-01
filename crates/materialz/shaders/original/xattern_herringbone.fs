outName = "Herringbone";
outCategory = "Pattern";

ParameterGroup = "Herringbone Pattern";
ParamSlider = "VCELLST", "Cells", "16", "1", "200";
ParamToolTip = "The vertical amount of cells.";
ParamSlider = "inRatio", "Ratio", "4", "1", "14";
ParamToolTip = "Width / height ratio of the individual tiles.";
ParamSlider = "ROUND", "Rounding", "0.0", "0.0", "1";
ParamToolTip = "Edge rounding of the tiles.";
ParamSlider = "inBEVEL", "Bevel", "0.0", "0.0", "1";
ParamToolTip = "Beveling of the tiles.";
ParamSlider = "inGap", "Gap", "0.2", "0.0", "1.0";
ParamToolTip = "Gap between the tiles in percent.";
ParamSlider = "inMaxRandRot", "Max Rotation", "0.0", "0.0", "10";
ParamToolTip = "Maximum random rotation per tile in degrees. A value of 0 means no rotation, a value of 10 means each tile gets randomly rotated up to 10 degrees.";
ParameterGroup = "Gradient", "false";
ParamBoolean = "inGradient", "Gradient", "0";
ParamToolTip = "Gives tiles a gradient, useful for generating uneveness through visually tilting the tile.";
ParamSlider = "inStrengthGradient", "Gradient Strength", "0.5", "0.0", "1";
ParamToolTip = "Strength of the gradient.";
ParameterGroup = "About";
ParamDescription = "Generates a herringbone pattern. Returns the cell id in the x component, the depth map in y and the angle of the tiles in degrees in the z component. Use the Split Operator to access the individual components.";

float RATIO = float(inRatio),
      CELL = round( float(VCELLST) / (2. * RATIO ) ) * 2. * RATIO,
      ROT0 = 0.,
      ROT1 = inMaxRandRot;
vec2  BEVEL = vec2(inBEVEL),
      GAP  = vec2(inGap/2.);
bool  BUMP = false;
bool GRAD = inGradient;
vec2  grad = vec2(1,0);
float grad_randA = 1.,
      grad_randD = 360.;

vec2  CYCLE = vec2( CELL );

vec3 O = vec3( 0 );
vec2 U = inUV;
vec2 R = inUV;

U *= CELL;
vec2 V = floor(U), W = vec2(RATIO,1);

float w =  mod( V.x+V.y, 2.*RATIO ) - RATIO;
bool vert = w<0.;
if (vert) U = U.yx, w += RATIO, V=V.yx;
ivec2 H = ivec2((V+V.y)/W);
H = H % ivec2(CYCLE/W);

vec2 S = vec2(w,0)+fract(U) - W/2.;

uvec3 x = uvec3(H,1);
uint k = 1103515245U;
x = ((x>>8U)^x.yzx)*k;
x = ((x>>8U)^x.yzx)*k;
x = ((x>>8U)^x.yzx)*k;
float rot = (vec3(x)*(1.0/float(0xffffffffU))).x;

float dir = ROT0 + ROT1 * (-1.+2.*rot);

mat2 matDir = mat2(sin(radians(dir) + vec4(1,0,2,1)*1.5707));
S *= matDir;

vec2 A = W/2.-GAP - abs(S),
      B  = A * 2. / BEVEL;
      float m = min(B.x,B.y);
if (A.x<ROUND && A.y<ROUND)
      m = (ROUND-length(ROUND-A)) *2./dot(BEVEL,normalize(ROUND-A));

O += clamp( m ,0.,1.);
if (GRAD) {
      uvec3 x = uvec3(H,3);
      const uint k = 1103515245U;
      x = ((x>>8U)^x.yzx)*k;
      x = ((x>>8U)^x.yzx)*k;
      x = ((x>>8U)^x.yzx)*k;
      vec3 rotHash = (vec3(x)*(1.0/float(0xffffffffU)));

      vec3 R = -1.+2.*rotHash;
      mat2 rot = mat2(sin(radians(grad_randD/2.*R.x) + vec4(1,0,2,1)*1.5707));

      grad *= rot * pow(grad_randA,R.y);
      O.rgb *= 3. - inStrengthGradient * 3. + dot(grad,S);
}

x = uvec3(H,2);
k = 1103515245U;
x = ((x>>8U)^x.yzx)*k;
x = ((x>>8U)^x.yzx)*k;
x = ((x>>8U)^x.yzx)*k;
float id = (vec3(x)*(1.0/float(0xffffffffU))).x;

O.r = id;
O.b = (vert ? 90. : 0.) + dir;

outColor = vec3( O.x, O.y, O.z );
