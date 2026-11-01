outName = "Decor Tiles";
outCategory = "Pattern";
inTerminal = "0", "Shape", "vec3(0)";
inDefaultTerminal = "Tile";

ParameterGroup = "Decor Tiles Pattern";
ParamSlider = "VCELLST", "Cells", "6", "1", "200";
ParamToolTip = "The vertical amount of cells.";
ParamSlider = "RATIO", "Ratio", "2.8", "0.01", "14.";
ParamToolTip = "Width / height ratio of the individual tiles.";
ParamSlider = "inGap", "Gap", "0.2", "0.0", "1.0";
ParamToolTip = "Gap between the tiles in percent.";
ParamSlider = "inMaxRandRot", "Max Rotation", "0.0", "0.0", "10";
ParamToolTip = "Maximum random rotation per tile in degrees. A value of 0 means no rotation, a value of 10 means each tile gets randomly rotated up to 10 degrees.";

ParameterGroup = "Shapes", "false";
ParamVector2 = "shapeN", "Amount", "1", "1", "0", "50";
ParamToolTip = "The amount of shapes in the x and y direction.";
ParamVector2 = "shapeGap", "Gap", "0.0", "0.0", "0", "1";
ParamToolTip = "The gap between the shapes.";

ParameterGroup = "About";
ParamDescription = "Generates a decor stone tile pattern. Needs an input shape (by default Tile). Returns the cell id in the x component, the depth map in y and the angle of the tiles in degrees in the z component. Use the Split Operator to access the individual components.";

<! FUNC hash !>
float <!hash!>( vec2 p ) { return fract(sin(dot(p,vec2(127.1,311.7)))*18.5453); }
<! FUNC END !>

<! FUNC voronoi !>
vec4 <!voronoi!>( vec2 u, vec2 CYCLE )
{
      #define <! disp !>(p) ( -(3.-1.)/2. + 3. * (p) )

      vec2  n = floor( u );
      float f = fract( u.x );

      vec2 m = vec2(1e5), left = vec2(-1e5),right = vec2(1e5), P;

      for( float g=-4.; g<=4.; g++ )
        {
            float o = <!hash!>(mod(n + vec2(g,0),CYCLE)),
                  d = g - f + <!disp!>(o);
            if( abs(d) < m.x ) m = vec2( abs(d), o);
            if ( d < 0.) {
                if (d>left.x) left.y = left.x, left.x = d;
                else if (d>left.y) left.y = d;
               }
            else         {
                if (d<right.x) right.y = right.x, right.x = d;
                else if (d<right.y) right.y = d;
               }

        }

    P =  -m.x==left.x ? vec2( left.x+left.y,   left.x+right.x ) / 2.
                      : vec2( left.x+right.x, right.x+right.y ) / 2.;
    P += u.x;
    return vec4(m, P.y-P.x, (P.x+P.y)/2. );
}
<! FUNC END !>

float CELL = float(VCELLST),
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

vec4 V = <!voronoi!>( U, CYCLE );
ivec2 H = ivec2( V.y*1e4, CYCLE==vec2(0) ? U.y : mod(U.y,CYCLE.y) );
vec2 S = vec2((U.x-V.w)*W.x,fract(U.y)-.5);

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
      W.x *= V.z;
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
O.b = dir + 90.*vert;

outColor = O;
