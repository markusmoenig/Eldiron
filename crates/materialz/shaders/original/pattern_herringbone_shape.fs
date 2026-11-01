outName = "Herringbone";
outCategory = "Pattern";
inTerminal = "0", "Shape", "vec3(0)";
inDefaultTerminal = "Tile";

ParameterGroup = "Herringbone Pattern";
ParamSlider = "VCELLST", "Cells", "16", "1", "200";
ParamToolTip = "The vertical amount of shapes.";
ParamSlider = "inRatio", "Ratio", "4", "1", "14";
ParamToolTip = "Width / height ratio of the individual shapes.";
ParamSlider = "inGap", "Gap", "0.2", "0.0", "1.0";
ParamToolTip = "Gap between the shapes in percent.";
ParamSlider = "inMaxRandRot", "Max Rotation", "0.0", "0.0", "10";
ParamToolTip = "Maximum random rotation per shape in degrees. A value of 0 means no rotation, a value of 10 means each shape gets randomly rotated up to 10 degrees.";
ParamSlider = "probability", "Probability", "1.0", "0.0", "1";
ParamToolTip = "The probability that a given shape is drawn, 1. always draws every shape, lower values draw less and less shapes.";

ParameterGroup = "Shapes", "false";
ParamVector2 = "shapeN", "Amount", "1", "1", "0", "50";
ParamToolTip = "The amount of shapes in the x and y direction.";
ParamVector2 = "shapeGap", "Gap", "0.0", "0.0", "0", "1";
ParamToolTip = "The gap between the shapes.";

ParameterGroup = "About";
ParamDescription = "Generates a herringbone pattern. Needs an input shape or pattern (by default Tile).</p><p>Returns the cell id in the <b>X</b> component, the depth map in <b>Y</b> and the angle of the shape in degrees in the <b>Z</b> component.</p><p>Use the <i>Split Operator</i> to access the individual components.";

float RATIO = float(inRatio),
      CELL = round( float(VCELLST) / (2. * RATIO ) ) * 2. * RATIO,
      ROT0 = 0.,
      ROT1 = inMaxRandRot;
vec2  GAP  = vec2(inGap/2.);

vec2  CYCLE = vec2( CELL );

vec3 O = vec3( 0 );
vec2 U = inUV + vec2( 0.5 );
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

x = uvec3(H,2);
k = 1103515245U;
x = ((x>>8U)^x.yzx)*k;
x = ((x>>8U)^x.yzx)*k;
x = ((x>>8U)^x.yzx)*k;
float id = (vec3(x)*(1.0/float(0xffffffffU))).x;

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

            O += InputFunc( inPos, S + vec2(i,j) * ofs, inNormal, inTime, vec3(iBox, shapeId ), vec3( float(vert), 0, 0), inInput3 );
        }

O.r = id;
O.b = (vert ? 90. : 0.) + dir;

float visible = step( id, probability);
O.g = mix( 0., O.g, visible );

outColor = vec3( O.x, O.y, O.z );
