outName = "Marble";
outCategory = "Pattern";

ParameterGroup = "Marble Pattern";
ParamSlider = "inCell", "Cells", "4", "1", "20";
ParamToolTip = "The vertical amount of cells.";
ParamSlider = "fractal_depth_i", "Layers", "3", "1", "6";
ParamToolTip = "The amount of crack layers.";
ParamSlider = "fractal_scale", "Scale per Layer", "1.5", "0.1", "4.";
ParamToolTip = "The downscale applied to each layer.";
ParamSlider = "CRACK_noise_scale", "Noise Scale", "1.0", "0.1", "4.";
ParamToolTip = "Scales the input noise for the shape.";
ParamSlider = "CRACK_noise_amp", "Noise Amp.", "0.67", "0.", "4.";
ParamToolTip = "Amplifies the noise. Useful for controlling the shape of the crack lines.";

ParameterGroup = "About";
ParamDescription = "Generates a marble pattern.";

// === Voronoi =====================================================
// --- Base Voronoi. inspired by https://www.shadertoy.com/view/MslGD8

<! FUNC voronoi !>
vec3 <! voronoi !>( vec2 u )  // returns len + id
{
#define <!VARIANT!> 1              // 1: amplifies Voronoi cell jittering
#if <!VARIANT!>
      float ofs = .5;          // jitter Voronoi centers in -ofs ... 1.+ofs
#else
      float ofs = 0.;
#endif

#define <!hash22!>(p)  fract( 18.5453 * sin( p * mat2(127.1,311.7,269.5,183.3)) )
#define <!disp!>(p) ( -ofs + (1.+2.*ofs) * <!hash22!>(p) )

    vec2 iu = floor(u), v;
	float m = 1e9,d;
#if <!VARIANT!>
    for( int k=0; k < 25; k++ ) {
        vec2  p = iu + vec2(k%5-2,k/5-2),
#else
    for( int k=0; k < 9; k++ ) {
        vec2  p = iu + vec2(k%3-1,k/3-1),
#endif
            o = <!disp!>(p),
      	      r = p - u + o;
		d = dot(r,r);
        if( d < m ) m = d, v = r;
    }

    return vec3( sqrt(m), v+u );
}
<! FUNC END !>

// --- Voronoi distance to borders. inspired by https://www.shadertoy.com/view/ldl3W8
<! FUNC voronoiB !>
vec3 <! voronoiB !>( vec2 u )  // returns len + id
{
#if <!VARIANT!>
      float ofs = .5;          // jitter Voronoi centers in -ofs ... 1.+ofs
#else
      float ofs = 0.;
#endif

      vec2 iu = floor(u), C, P;
      float m = 1e9,d;
#if <!VARIANT!>
    for( int k=0; k < 25; k++ ) {
        vec2  p = iu + vec2(k%5-2,k/5-2),
#else
    for( int k=0; k < 9; k++ ) {
        vec2  p = iu + vec2(k%3-1,k/3-1),
#endif
              o = <!disp!>(p),
      	      r = p - u + o;
		d = dot(r,r);
        if( d < m ) m = d, C = p-iu, P = r;
    }

    m = 1e9;

    for( int k=0; k < 25; k++ ) {
        vec2 p = iu+C + vec2(k%5-2,k/5-2),
		     o = <!disp!>(p),
             r = p-u + o;

        if( dot(P-r,P-r)>1e-5 )
        m = min( m, .5*dot( (P+r), normalize(r-P) ) );
    }

    return vec3( m, P+u );
}
<! FUNC END !>

<! FUNC hash3 !>
vec3 hash3( uvec3 x )
{
#   define <!scramble!>  x = ( (x>>8U) ^ x.yzx ) * 1103515245U // GLIB-C const
    <!scramble!>; <!scramble!>; <!scramble!>;
    return vec3(x) / float(0xffffffffU);
}
<! FUNC END !>

<! FUNC noise2 !>
float <! noise2 !>(vec2 p) {
#define <!hash21!>(p) fract(sin(dot(p,vec2(127.1,311.7)))*43758.5453123)
      int MOD = 1;
      vec2 i = floor(p);
      vec2 f = fract(p); f = f*f*(3.-2.*f); // smoothstep

      float v= mix( mix(<!hash21!>(i+vec2(0,0)),<!hash21!>(i+vec2(1,0)),f.x),
                  mix(<!hash21!>(i+vec2(0,1)),<!hash21!>(i+vec2(1,1)),f.x), f.y);
	return   MOD==0 ? v
	       : MOD==1 ? 2.*v-1.
           : MOD==2 ? abs(2.*v-1.)
                    : 1.-abs(2.*v-1.);
}
<! FUNC END !>

<! FUNC fbm2 !>
float <! fbm2 !>(vec2 p) {
#define <!rot!>(a) mat2(cos(a),-sin(a),sin(a),cos(a))
    float v = 0.,  a = .5;
    mat2 R = <!rot!>(.37);

    for (int i = 0; i < 9; i++, p*=2.,a/=2.)
        p *= R,
        v += a * <!noise2!>(p);

    return v;
}
<! FUNC END !>

<! FUNC fbm22 !>
vec2 <! fbm22 !>(vec2 p) {
#define <! noise22 !>(p) vec2(<!noise2!>(p),<!noise2!>(p+17.7))
    vec2 v = vec2(0);
    float a = .5;
    mat2 R = <!rot!>(.37);

    for (int i = 0; i < 6; i++, p*=2.,a/=2.)
        p *= R,
        v += a * <! noise22 !>(p);

    return v;
}
<! FUNC END !>

<! FUNC mfbm22 !>
vec2 <! mfbm22 !>(vec2 p) {  // multifractal fbm
    vec2 v = vec2(1);
    float a = .5;
    mat2 R = <!rot!>(.37);

    for (int i = 0; i < 6; i++, p*=2.,a/=2.)
        p *= R,
        //v *= 1.+noise22(p);
          v += v * a * <! noise22 !>(p);

    return v-1.;
}
<! FUNC END !>

float RATIO = 1.,              // stone length/width ratio
      CELL = float(inCell),
      fractal_depth = float(fractal_depth_i),
      CRACK_bevel = 50.,       // cracks profile = \_ = bevel,width
      CRACK_width = .0;

vec2 U = inUV * CELL + inInput1.z * 12323.1211;
vec3 O = vec3( 0 );
vec3 H0;

for(float i=0.; i<fractal_depth ; i++) {
      vec2 V =  U / vec2(RATIO,1),                  // voronoi cell shape
            D = CRACK_noise_amp * <!fbm22!>(U/CRACK_noise_scale) * CRACK_noise_scale;
      vec3  H = <!voronoiB!>( V + D ); if (i==0.) H0=H;
      float d = H.x;                                // distance to cracks
      d = clamp( CRACK_bevel * (d-CRACK_width) ,0., 1.);

      O += vec3(1.-d) / exp2(i);  // NB: if no fractal we just have O = d
      U *= fractal_scale * <!rot!>(.37);
}

O = 1.-O;

vec3 color =vec3( O );

outColor = color;
