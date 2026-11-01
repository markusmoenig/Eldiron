outName = "Random";
outCategory = "Pattern";
inTerminal = "0", "Shape", "vec3(0)";
inDefaultTerminal = "Tile";

ParameterGroup = "Random Pattern";
ParamSlider = "VCELLST", "Cells", "6", "1", "100";
ParamToolTip = "The vertical amount of cells.";
ParamSlider = "RATIO", "Ratio", "1.0", "0.01", "10";
ParamToolTip = "Cell length / width ratio.";

ParamList = "BLEND", "Blend", "0", "Addition, Maximum";

ParamSlider = "DEPTH", "Layers", "1", "1", "3";
ParamToolTip = "Amount of shape layers.";

ParamSlider = "LACUNARITY", "Lacunarity", "4.0", "0.0", "10.0";
ParamToolTip = "The amount the shapes get smaller per layer.";

ParameterGroup = "Shapes", "false";
ParamSlider = "RADIUSA", "Scale", "0.5", "0.01", "3";
ParamToolTip = "Scale of the cells.";
ParamSlider = "RADIUS_D", "Size Distribution", "1.0", "0.0", "2";
ParamToolTip = "Size distribution of the cells. Higher values vary the size more.";
ParamSlider = "AMP_D", "Intensity Distribution", "0.0", "0.0", "2";
ParamToolTip = "Intensity distribution of the cells. Higher values vary the intensity of cells more.";
ParamSlider = "NB", "Shapes per Cell", "1", "1", "3";
ParamToolTip = "Amount of shapes per cell.";
ParamSlider = "ROTshape1", "Random Rot.", "180.0", "0.0", "180";
ParamToolTip = "Maximum random rotation in degrees.";

ParameterGroup = "About";
ParamDescription = 'Creates a random distribution of the input shape.</p><p>Returns the cell id in the <b>X</b> component, the depth map in <b>Y</b> and the angle of the shape in degrees in the <b>Z</b> component.</p><p>Use the <i>Split Operator</i> to access the individual components.';

const int Addition = 0, Maximum = 1;

#define <!VARIANT!> 0
float ofs = 0.;

float CELL = float( VCELLST );
vec2 CYCLE = vec2( round(CELL / RATIO), CELL );

float       ROT0 = 0.;

<! FUNC hash3 !>
vec3 <!hash3!>( uvec3 x )
{
    #define <!scramble!> x = ( (x>>8U) ^ x.yzx ) * 1103515245U
    <!scramble!>; <!scramble!>; <!scramble!>;
    return vec3(x) / float(0xffffffffU) + 1e-30;
}
<! FUNC END !>

<! FUNC hash3f !>
vec3 <!hash3f!>(vec3 x) {
    return <!hash3!>(uvec3( x * float(0xffffffffU) ) );
}
<! FUNC END !>

#define <!rot!>(a) mat2(sin(radians(a) + vec4(1,0,2,1)*1.5707))
#define <!hash3i!>(x) <!hash3!>(uvec3(ivec3(floor(x))))

vec2 U = (inUV + vec2( 0.5 ));
float O = 0.;

vec2 W = vec2(RATIO,1);
U *= CELL / W;
vec2 U0 = U;

float dir = 0.,  id = -1.;
float s = 1., t=0., height;

for (int i=0; i<DEPTH; i++) {
    vec2 u = U, iu = floor(u);
    height = 0.;
    #if <!VARIANT!>
    for( int k=0; k < 25; k++ ) {
        vec2 p = iu + vec2(k%5-2,k/5-2);
    #else
    for( int k=0; k < 9; k++ ) {
        vec2 p = iu + vec2(k%3-1,k/3-1);
    #endif
        for( int n=0; n<NB; n++) {
            vec3 H = <!hash3i!>(vec3(CYCLE==vec2(0)?p:mod(p,CYCLE),n));
            vec2
                    o = ( -ofs + (1.+2.*ofs) * H.xy ),
                    r = p - u + o;

            r *= W;
            r /= RADIUSA * (1.+RADIUS_D*(2.*H.z-1.));
            H = <!hash3f!>(H);
            float id0 = H.x,
                    dir0 = ROT0 + ROTshape1 * (-1.+2.*H.y);
            r *= <!rot!>(dir0);

            float a = .5 + AMP_D*(2.*H.z-1.),
                    h = a* InputFunc( inPos, r, inNormal, inTime, vec3(1,1, id0 ), vec3(0, 0, 0), inInput3 ).x;

            if (BLEND == 0)  height += h ;
            else             height = max(height, h);
            if ( (i==0 || id<0.)  &&
                    h>0. && h==height) dir = dir0, id = id0;
        }
    }
    O += s * height;
    U *= LACUNARITY; CYCLE *= LACUNARITY;
#if 1
    t += s; s /= LACUNARITY;
#endif
    }

if (BLEND==1) O /= t;

outColor = vec3( id, O, dir );

