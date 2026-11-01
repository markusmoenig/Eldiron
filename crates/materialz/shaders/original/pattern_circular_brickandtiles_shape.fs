outName = "Circular Brick & Tiles";
outCategory = "Pattern";
inTerminal = "0", "Shape", "vec3(0)";
inDefaultTerminal = "Tile";

ParameterGroup = "Circular Brick & Tiles Pattern";
ParamSlider = "VCELLNT", "Cells", "20", "1", "200";
ParamToolTip = "The vertical amount of shapes.";
ParamSlider = "inRatio", "Ratio", "4.0", "0.01", "14.";
ParamToolTip = "Width / height ratio of the individual shapes.";
ParamSlider = "inGap", "Gap", "0.1", "0.0", "1.0";
ParamToolTip = "Gap between the shapes in percent.";
ParamSlider = "inHOLLOW", "Center Size", "1", "0", "20";
ParamToolTip = "The size of the empty center in shapes.";
ParamSlider = "inMaxRandRot", "Max Rotation", "0.0", "0.0", "10";
ParamToolTip = "Maximum random rotation per shape in degrees. A value of 0 means no rotation, a value of 10 means each shape gets randomly rotated up to 10 degrees.";
ParamBoolean = "BRICK", "Brick", "1";
ParamToolTip = "Switch between shape and brick pattern.";
ParamSlider = "probability", "Probability", "1.0", "0.0", "1";
ParamToolTip = "The probability that a given shape is drawn, 1. always draws every shape, lower values draw less and less shapes.";

ParameterGroup = "Offsets", "false";
ParamSlider = "OFFSET", "Offset", "0.0", "0.0", "1.0";
ParamToolTip = "Offset between circles.";
ParamBoolean = "RANDofs", "Random Offsets", "1";
ParamToolTip = "Adds a random offset to the offsets between circles to reduce visible tiling.";

ParameterGroup = "Shapes", "false";
ParamVector2 = "shapeN", "Amount", "1", "1", "0", "50";
ParamToolTip = "The amount of shapes in the x and y direction.";
ParamVector2 = "shapeGap", "Gap", "0.0", "0.0", "0", "1";
ParamToolTip = "The gap between the shapes.";

ParameterGroup = "About";
ParamDescription = "Generates a circular brick or tile pattern. Needs an input shape or pattern (by default Tile).</p><p>Returns the cell id in the <b>X</b> component, the depth map in <b>Y</b> and the angle of the shape in degrees in the <b>Z</b> component.</p><p>Use the <i>Split Operator</i> to access the individual components.";

<! FUNC hash3 !>
vec3 <!hash3!>( uvec3 x )
{
      #define <!scramble!>  x = ( (x>>8U) ^ x.yzx ) * 1103515245U
      <!scramble!>; <!scramble!>; <!scramble!>;
      return vec3(x) / float(0xffffffffU);
}
<! FUNC END !>

<! FUNC hash3f !>
vec3 <!hash3f!>(vec3 x) {
      return <!hash3!>(uvec3( x * float(0xffffffffU) ) );
}
<! FUNC END !>

#define <!rot!>(a) mat2(sin(radians(a) + vec4(1,0,2,1)*1.5707))

float CELL = float(VCELLNT),
      RATIO = float(inRatio),
      ROT0 = 0.,
      ROT1 = inMaxRandRot,
      HOLLOW = float(inHOLLOW);
vec2  CYCLE = vec2( round(CELL / RATIO), CELL );
vec2  GAP  = vec2(inGap/2.);
bool  gap = false;
float vert = 0.;

vec3 O = vec3( 0 );
vec2 U = inUV;

float l = length(U)*CELL, a = atan(U.y,U.x), k;
U = vec2( a*(floor(l)+.5), l );
if (U.y<HOLLOW) gap = true;
k = 6.28*(floor(l)+.5)/RATIO;
U.x *= (floor(k)+(0.)) / k;
if (RANDofs) U.x += <!hash3!>(uvec3(l,1,12)).x * RATIO;
U.x += OFFSET*floor(l);
CYCLE = vec2(floor(k),1e5);

vec2 W = vec2(RATIO,1);

 U /= W;

if (BRICK) U.x += .5* mod(floor(U.y),2.);


ivec2 H = ivec2( CYCLE==vec2(0) ? U : mod(U,CYCLE) );
vec2 S = W* (fract(U) - 1./2.);

float id = <!hash3!>(uvec3(H,2)).r, dir = ROT0 + ROT1 * (-1.+2.*<!hash3!>(uvec3(H,1)).x);
S *= <!rot!>(dir);


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

O.r = gap ? 1. : id;
O.b = dir;

float visible = step( id, probability);
O.g = mix( 0., O.g, visible );

outColor = O;
