outName = "Cracks";
outCategory = "Pattern";

ParameterGroup = "Cracks Pattern";
ParamSlider = "inCell", "Cells", "4", "1", "20";
ParamToolTip = "The vertical amount of cells.";
ParamList = "inType", "Type", "1", "Crest, Fault";
ParamSlider = "RATIO", "Ratio", "2.0", "0.01", "10";
ParamToolTip = "Tile length / width ratio.";
ParamSlider = "probability", "Probability", "1.0", "0.0", "1";
ParamToolTip = "The probability that a given shape is drawn, 1. always draws every shape, lower values draw less and less shapes.";
ParameterGroup = "Cell", "false";
ParamSlider = "STONE_slope", "Cell Slope", "0.3", "-1.0", "1.0";
ParamSlider = "STONE_height", "Cell Height", "1.0", "0.0", "1.0";
ParamSlider = "STONE_profile", "Profile", "1.0", "0.0", "2.0";
ParameterGroup = "Crack", "false";
ParamSlider = "CRACK_zebra_scale", "Zebra Scale", "1.5", "0.0", "10.0";
ParamSlider = "CRACK_zebra_amp", "Zebra Amp", "1.7", "0.0", "10.0";
ParamSlider = "CRACK_profile", "Profile", "0.2", "0.0", "1.0";
ParamSlider = "CRACK_slope", "Slope", "1.4", "0.0", "2.0";
ParamSlider = "CRACK_width", "Width", "0.0", "0.0", "0.1";
ParameterGroup = "About";
ParamDescription = "Generates a pattern of random cracks.</p><p>Returns the cell id in the <b>X</b> component and the depth map in <b>Y</b> component.</p><p>Use the <i>Split Operator</i> to access the individual components.";

const int Crest = 0, Fault = 1;
float ofs = 0.;
int FAULT = inType;
float CELL = float(inCell);

vec2 U = inUV * CELL + inInput1.z * 12323.1211;
vec2 V =  U / vec2(RATIO,1);
vec2 fbm22_in = vec2( CRACK_zebra_scale*U );

// --- fbm22
vec2 p = fbm22_in;
vec2 v = vec2(0);
float a = .5;

mat2 R = mat2(cos(.37),-sin(.37),sin(.37),cos(.37));
for (int i = 0; i < 9; i++, p*=2.,a/=2.) {
    p *= R;

    // --- Noise2 p: x
    const int MOD = 1;  // type of Perlin noise
    vec2 i1 = floor(p);
    vec2 f = fract(p); f = f*f*(3.-2.*f);
    float v1= mix( mix(fract(sin(dot(i1+vec2(0,0),vec2(127.1,311.7)))*43758.5453123), fract(sin(dot(i1+vec2(1,0),vec2(127.1,311.7)))*43758.5453123),f.x),
                  mix(fract(sin(dot(i1+vec2(0,1),vec2(127.1,311.7)))*43758.5453123), fract(sin(dot(i1+vec2(1,1),vec2(127.1,311.7)))*43758.5453123),f.x), f.y);
    float noise22_x = MOD==0 ? v1 : MOD==1 ? 2.*v1-1. : MOD==2 ? abs(2.*v1-1.) : 1.-abs(2.*v1-1.);

    // --- Noise2 p + 17.7: y
    i1 = floor(p+17.7);
    f = fract(p+17.7); f = f*f*(3.-2.*f);
    v1= mix( mix(fract(sin(dot(i1+vec2(0,0),vec2(127.1,311.7)))*43758.5453123), fract(sin(dot(i1+vec2(1,0),vec2(127.1,311.7)))*43758.5453123),f.x),
                  mix(fract(sin(dot(i1+vec2(0,1),vec2(127.1,311.7)))*43758.5453123), fract(sin(dot(i1+vec2(1,1),vec2(127.1,311.7)))*43758.5453123),f.x), f.y);
    float noise22_y = MOD==0 ? v1 : MOD==1 ? 2.*v1-1. : MOD==2 ? abs(2.*v1-1.) : 1.-abs(2.*v1-1.);

    vec2 noise22 = vec2( noise22_x, noise22_y );

    v += a * noise22;
}

vec2 fbm22 = v;
vec2 D = fbm22 / CRACK_zebra_scale / CRACK_zebra_amp;

// --- voronoiB

vec2 u = V + D;
vec2 iu = floor(u), C, P;
float m = 1e9,d;
for( int k=0; k < 9; k++ ) {
    vec2  p = iu + vec2(k%3-1,k/3-1),
            o = -ofs + (1.+2.*ofs) * fract( 18.5453 * sin( p * mat2(127.1,311.7,269.5,183.3)) ),
            r = p - u + o;
    d = dot(r,r);
    if( d < m ) m = d, C = p-iu, P = r;
}

m = 1e9;

for( int k=0; k < 25; k++ ) {
    vec2 p = iu+C + vec2(k%5-2,k/5-2),
            o = -ofs + (1.+2.*ofs) * fract( 18.5453 * sin( p * mat2(127.1,311.7,269.5,183.3)) ),
            r = p-u + o;

    if( dot(P-r,P-r)>1e-5 )
    m = min( m, .5*dot( (P+r), normalize(r-P) ) );
}

vec3 H_voronoiB = vec3( m, P+u );

// --- voronoi

u = V;
iu = floor(u);
m = 1e9;
for( int k=0; k < 9; k++ ) {
    vec2  p = iu + vec2(k%3-1,k/3-1),
            o = -ofs + (1.+2.*ofs) * fract( 18.5453 * sin( p * mat2(127.1,311.7,269.5,183.3)) ),
            r = p - u + o;
    d = dot(r,r);
    if( d < m ) m = d, v = o;
}

float r = vec2( sqrt(m), v.x + 256.*v.y ).x;

float s = STONE_height-(STONE_slope)*pow(r,STONE_profile);

d = min( 1., CRACK_slope * pow(max(0.,H_voronoiB.x-CRACK_width),CRACK_profile) );
float crack = FAULT==1 ? d * s : mix(1.,s, d);

uvec3 x = uvec3(H_voronoiB.yz,1);
uint k = 1103515245U;
x = ((x>>8U)^x.yzx)*k;
x = ((x>>8U)^x.yzx)*k;
x = ((x>>8U)^x.yzx)*k;
float id = (vec3(x)*(1.0/float(0xffffffffU))).x;

float visible = step( id, probability);
crack = mix( 0., crack, visible );

outColor = vec3( id, crack, 0 );

