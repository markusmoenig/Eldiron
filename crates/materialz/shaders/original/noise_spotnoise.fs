outName = "Spot Noise";
outCategory = "2D Noise";

ParameterGroup = "Spot Noise";
ParamSlider = "VCELLST", "Cells", "20", "1", "40";
ParamToolTip = "The vertical amount of cells.";
ParamSlider = "RATIO", "Ratio", "1.0", "0.01", "100";
ParamToolTip = "Tile length / width ratio.";

ParamList = "SHAPE", "Profile", "1", "Blob, Gaussian, Hemisphere, Antisphere";
ParamList = "BLEND", "Blend", "0", "Addition, Maximum";

ParameterGroup = "Spots", "false";
ParamSlider = "RADIUSA", "Radius", "1.0", "0.01", "3";
ParamToolTip = "Radius of the spots.";
ParamSlider = "RADIUS_D", "Size Distribution", "0.0", "0.0", "2";
ParamToolTip = "Size distribution of the spots.";
ParamSlider = "NB", "Dots per Cell", "1", "1", "3";
ParamToolTip = "Amount of dots per cell.";

ParameterGroup = "About";
ParamDescription = 'Creates random spots of configurable styles and sizes.';

const int Blob = 0, Gaussian = 1, Hemisphere = 2, Antisphere = 3;
const int Addition = 0, Maximum = 1;

#define <!VARIANT!> 0
float ofs = 0.;

float CELL = float( VCELLST ),
     AMP_D = .0;

int DEPTH = 1;

vec2 CYCLE = vec2( round(CELL / RATIO), CELL );

<! FUNC profile !>
float <!profile!>( int SHAPE, float d) {
    switch(SHAPE) {
        case 0: return smoothstep(1.,0.,d);
        case 1: return exp(-.5*d*d*6.);
        case 2: d*=1.5; return sqrt(max(0.,1.-d*d));
        case 3: d = max(0.,1.-d); return 1. - sqrt(1.-d*d );
    }
}
<! FUNC END !>

<! FUNC hash3 !>
vec3 <!hash3!>( uvec3 x )
{
    #define <!scramble!> x = ( (x>>8U) ^ x.yzx ) * 1103515245U
    <!scramble!>; <!scramble!>; <!scramble!>;
    return vec3(x) / float(0xffffffffU) + 1e-30;
}
<! FUNC END !>

#define <!hash3i!>(x) <!hash3!>(uvec3(ivec3(floor(x))))

vec2 U = (inUV + vec2( 0.5 ));
float O = 0.;

U *= CELL/vec2(RATIO,1);
vec2 U0 = U;
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
            float d = length(r)/(RADIUSA+RADIUS_D*(2.*H.z-1.)),
                    a = .5 + AMP_D*(2.*H.z-1.),
                    h = a *  <!profile!>(SHAPE, d);
            if (BLEND == 0)  height += h ;
            else             height = max(height, h);
        }
    }
    O += s * height;
    U *= 2.;  t += s; s /= 2.;
}
O /= t;
if (BLEND==1) O*=2.;

outColor = vec3( O );
