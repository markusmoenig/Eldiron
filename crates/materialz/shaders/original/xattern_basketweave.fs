outName = "Basket Weave";
outCategory = "Pattern";

ParameterGroup = "Basket Weave Pattern";
ParamSlider = "VCELLST", "Cells", "12", "1", "200";
ParamToolTip = "The vertical amount of cells.";
ParamSlider = "ROUND", "Rounding", "0.0", "0.0", "1";
ParamToolTip = "Edge rounding of the shapes.";
ParamSlider = "inBEVEL", "Bevel", "0.0", "0.0", "1";
ParamToolTip = "Beveling of the shapes.";
ParamSlider = "inGap", "Gap", "0.2", "0.0", "1.0";
ParamToolTip = "Gap between the shapes in percent.";
ParamSlider = "inMaxRandRot", "Max Rotation", "0.0", "0.0", "10";
ParamToolTip = "Maximum random rotation per tile in degrees. A value of 0 means no rotation, a value of 10 means each tile gets randomly rotated up to 10 degrees.";
ParameterGroup = "Gradient", "false";
ParamBoolean = "inGradient", "Gradient", "0";
ParamToolTip = "Gives shapes a gradient, useful for generating uneveness through visually tilting the tile.";
ParamSlider = "inStrengthGradient", "Gradient Strength", "0.5", "0.0", "1";
ParamToolTip = "Strength of the gradient.";
ParameterGroup = "Tile Grid", "false";
ParamVector2 = "SUPERCELL", "Grid", "0", "0", "0", "20";
ParamToolTip = "Shapes can be grouped into a grid. The grid gap will be applied after every x, y grid shape.";
ParamVector2 = "SUPERGAP", "Grid Gap", "0.0", "0.0", "0", "10";
ParamToolTip = "Shapes can be grouped into a grid. The grid gap will be applied after every x, y grid shape.";
ParameterGroup = "About";
ParamDescription = "Generates a basket weave pattern. Returns the cell id in the x component, the depth map in y and the angle of the shape in degrees in the z component. Use the Split Operator to access the individual components.";

float CELL = float(VCELLST),
      RATIO = 2.,
      ROT0 = 0.,
      ROT1 = inMaxRandRot;
vec2  GAP  = vec2(inGap/2.);
vec2  BEVEL = vec2(inBEVEL);
vec2  CYCLE = vec2( round(CELL / RATIO), CELL );
bool  gap = false;
float vert = 0.;

bool GRAD = inGradient;
vec2  grad = vec2(1,0);
float grad_randA = 2.,
      grad_randD = 360.;

vec3 O = vec3( 0 );
vec2 U = inUV;
vec2 R = inUV;

vec2 W = vec2(RATIO,1);
U *= CELL;

vec2 SUPERRATIO = SUPERCELL * RATIO;

if (SUPERRATIO!=vec2(0)) {
      vec2 iU = floor(U / SUPERRATIO) * SUPERRATIO, fU = mod(U,SUPERRATIO);
      fU = (fU-SUPERGAP) / (SUPERRATIO-2.*SUPERGAP);
      if ( fU !=  clamp(fU,0.,1.) ) gap = true;
      fU = clamp(fU,0.,1.);
      U =  iU + fU * SUPERRATIO;
}

vec2 iU = floor(U/ RATIO);
if ( mod(iU.x+iU.y,2.) > 0. ) vert = 1.;
if (vert==1.) U = U.yx;
U /= W;

// ivec2 H = ivec2(U);
ivec2 H = ivec2( CYCLE==vec2(0) ? U : mod(U,CYCLE) ); // vec2 tile id
vec2 S = W* (fract(U) - 1./2.);

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

if ( !gap ) O += clamp( m ,0.,1.);

if ( GRAD ) {
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
O.g = clamp( O.g, 0., 1. );
O.b = dir + 90.*vert;

outColor = O;
