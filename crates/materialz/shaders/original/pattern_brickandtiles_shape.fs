outName = "Brick & Tiles";
outCategory = "Pattern";
inTerminal = "0", "Shape", "vec3(0)";
inDefaultTerminal = "Tile";

ParameterGroup = "Brick & Tiles Pattern";
ParamSlider = "VCELLST", "Cells", "20", "1", "200";
ParamToolTip = "The vertical amount of shapes.";
ParamSlider = "inRatio", "Ratio", "4.0", "0.01", "14.";
ParamToolTip = "Width / height ratio of the individual shapes.";
ParamSlider = "inGap", "Gap", "0.1", "0.0", "1.0";
ParamToolTip = "Gap between the shapes in percent.";
ParamSlider = "inMaxRandRot", "Max Rotation", "0.0", "0.0", "10";
ParamToolTip = "Maximum random rotation per shape in degrees. A value of 0 means no rotation, a value of 10 means each shape gets randomly rotated up to 10 degrees.";
ParamBoolean = "BRICK", "Brick", "1";
ParamToolTip = "Switch between shape and brick pattern.";
ParamSlider = "probability", "Probability", "1.0", "0.0", "1";
ParamToolTip = "The probability that a given shape is drawn, 1. always draws every shape, lower values draw less and less shapes.";

ParameterGroup = "Shapes", "false";
ParamVector2 = "shapeN", "Amount", "1", "1", "0", "50";
ParamToolTip = "The amount of shapes in the x and y direction.";
ParamVector2 = "shapeGap", "Gap", "0.0", "0.0", "0", "1";
ParamToolTip = "The gap between the shapes.";

ParameterGroup = "Shape Grid", "false";
ParamVector2 = "SUPERCELL", "Grid", "0", "0", "0", "20";
ParamToolTip = "Shapes can be grouped into a grid. The grid gap will be applied after every x, y grid shape. Only applicable for tiles, not bricks.";
ParamVector2 = "SUPERGAP", "Grid Gap", "0.0", "0.0", "0", "10";
ParamToolTip = "Shapes can be grouped into a grid. The grid gap will be applied after every x, y grid shape. Only applicable for tiles, not bricks.";
ParameterGroup = "About";
ParamDescription = "Generates a brick or tile pattern. Needs an input shape or pattern (by default Tile).</p><p>Returns the cell id in the <b>X</b> component, the depth map in <b>Y</b> and the angle of the shape in degrees in the <b>Z</b> component.</p><p>Use the <i>Split Operator</i> to access the individual components.";

float CELL = float(VCELLST),
      RATIO = float(inRatio),
      ROT0 = 0.,
      ROT1 = inMaxRandRot;
vec2  GAP  = vec2(inGap/2.);
vec2  CYCLE = vec2( round(CELL / RATIO), CELL );
bool  gap = false;
float vert = 0.;

vec3 O = vec3( 0 );
vec2 U = inUV;
vec2 R = inUV;

vec2 W = vec2(RATIO,1);
U *= CELL/W;

if (BRICK) U.x += .5* mod(floor(U.y),2.);
else if (SUPERCELL!=vec2(0)) {
      vec2 iU = floor(U / SUPERCELL) * SUPERCELL, fU = mod(U,SUPERCELL);
      fU = (fU-SUPERGAP) / (SUPERCELL-2.*SUPERGAP);
      if ( fU !=  clamp(fU,0.,1.) ) gap = true;
      fU = clamp(fU,0.,1.);
      U =  iU + fU * SUPERCELL;
}

ivec2 H = ivec2( CYCLE==vec2(0) ? U : mod(U,CYCLE) );
vec2 S = W* (fract(U) - 1./2.);

uvec3 x = uvec3(H,2);
uint k = 1103515245U;
x = ((x>>8U)^x.yzx)*k;
x = ((x>>8U)^x.yzx)*k;
x = ((x>>8U)^x.yzx)*k;
float id = (vec3(x)*(1.0/float(0xffffffffU))).x;

x = uvec3(H,1);
k = 1103515245U;
x = ((x>>8U)^x.yzx)*k;
x = ((x>>8U)^x.yzx)*k;
x = ((x>>8U)^x.yzx)*k;
float rot = (vec3(x)*(1.0/float(0xffffffffU))).x;

float dir = ROT0 + ROT1 * (-1.+2.*rot);
mat2 matDir = mat2(sin(radians(dir) + vec4(1,0,2,1)*1.5707));

S *= matDir;

if ( !gap ) {
      vec2 Box = vec2(  W-2.*GAP );
      vec2 N = shapeN-1., ofs = Box/shapeN;
      vec2 iBox = Box/shapeN - shapeGap;
      if ( shapeN.x > 1.0 ) iBox = vec2(min(iBox.x,iBox.y));
      for (float n=1., j = -N.y/2.; j <=N.y/2.; j++)
            for (float i = -N.x/2.; i <=N.x/2.; i++, n++) {

                  uvec3 x = uvec3( vec3(id,.9/n++,0) * float(0xffffffffU) );
                  const uint k = 1103515245U;
                  x = ((x>>8U)^x.yzx)*k;
                  x = ((x>>8U)^x.yzx)*k;
                  x = ((x>>8U)^x.yzx)*k;
                  float shapeId = (vec3(x)*(1.0/float(0xffffffffU))).x;

                  O += InputFunc( inPos, S + vec2(i,j) * ofs, inNormal, inTime, vec3(iBox, shapeId ), vec3(vert, 0, 0), inInput3 );
            }
}

O.r = id;
O.b = dir;

float visible = step( id, probability);
O.g = mix( 0., O.g, visible );

outColor = O;
