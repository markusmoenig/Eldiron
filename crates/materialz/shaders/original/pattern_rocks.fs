outName = "Rocks";
outCategory = "Pattern";

ParameterGroup = "Rock Pattern";
ParamSlider = "VCELLST", "Cells", "4", "1", "20";
ParamToolTip = "The vertical amount of cells.";
ParamSlider = "RATIO", "Ratio", "1.0", "0.01", "10";
ParamToolTip = "Tile length / width ratio.";

ParamList = "BLEND", "Blend", "0", "Addition, Maximum";

ParameterGroup = "Rocks", "false";
ParamSlider = "ROCK_radiusA", "Size", "0.6", "0.0", "1.0";
ParamSlider = "ROCK_radiusD", "Spread", "0.2", "0.0", "1.0";
ParamSlider = "BEVEL", "Bevel", "0.1", "0.0", "1.0";
ParamSlider = "inLayers", "Layers", "3", "1", "10";
ParamSlider = "ROCK_noise_scale", "Noise Scale", "1.0", "0.0", "10";
ParamSlider = "ROCK_noise_amp", "Noise Amp.", "0.37", "0.0", "10";

ParamSlider = "ROCK_nb", "Rocks per Cell", "1", "1", "10";
ParamVector2 = "gradA", "Gradient", "0.5", "0.0", "-1000", "1000";
ParamSlider = "grad_randA", "Gradient Spread", "2.0", "0.0", "10.0";

ParameterGroup = "About";
ParamDescription = "Generates a rock pattern.</p><p>Returns the cell id in the <b>X</b> component, the depth map in <b>Y</b> and the angle of the cell in degrees in the <b>Z</b> component.</p><p>Use the <i>Split Operator</i> to access the individual components.";

float CELL = float( VCELLST ),                   // number of cells along height
      ROCK_depth = float( inLayers );             // number of rock layers (of smaller size)

vec2 CYCLE = vec2( round(CELL / RATIO), CELL );

<! FUNC hash3 !>
vec3 <!hash3!>( uvec3 x )
{
    #define <!scramble!> x = ( (x>>8U) ^ x.yzx ) * 1103515245U
    <!scramble!>; <!scramble!>; <!scramble!>;
    return vec3(x) / float(0xffffffffU) + 1e-30;
}
<! FUNC END !>

#define <!hash3i!>(x) <!hash3!>(uvec3(ivec3(floor(x))))

// === Voronoi =====================================================

#define <!hash22!>(p) fract( 18.5453 * sin( p * mat2(127.1,311.7,269.5,183.3)) )
#define <!disp!>(p) ( 0.0 + (1.+2.*0.0) * <!hash22!>(p) )

// === pseudo Perlin noise =============================================

// --- 2D
<! FUNC noise2 !>
float <!noise2!>(vec2 p) {
    #define <!hash21!>(p) fract(sin(dot(p,vec2(127.1,311.7)))*43758.5453123)
    #define <!rot!>(a) mat2(cos(a),-sin(a),sin(a),cos(a))

    const int MOD = 1;  // type of Perlin noise

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
float <!fbm2!>(vec2 p) {
    float v = 0.,  a = .5;
    mat2 R = <!rot!>(.37);

    for (int i = 0; i < 9; i++, p*=2.,a/=2.)
        p *= R,
        v += a * <!noise2!>(p);

    return v;
}
<! FUNC END !>

<! FUNC fbm22 !>
vec2 <!fbm22!>(vec2 p) {
    #define <!noise22!>(p) vec2(<!noise2!>(p),<!noise2!>(p+17.7))
    vec2 v = vec2(0);
    float a = .5;
    mat2 R = <!rot!>(.37);

    for (int i = 0; i < 5; i++, p*=2.,a/=2.)
        p *= R,
        v += a * <!noise22!>(p);

    return v;
}
<! FUNC END !>

// ======================================================

// rock texturing user function. in: rotated CELL coords. out: [-1,1]
<! FUNC func !>
float <!func!>(vec2 U) {
    return 0.;                         // no modulation
  //return .1*sin(6.28*5.*U.x);        // dummy texturing
  //return texture(iChannel1, U.yx).x;
}
<! FUNC END !>

vec2 U = (inUV) * CELL + vec2( 0.5 );
vec2 U0 = U, id0=vec2(0);
float O = 0.;
float s = .5, height, dir = 0., ICslope = 1.;
const int Addition = 0, Maximum = 1;

#define <!VARIANT!> 0                  // 1: enlarge tested region around cell

for (float i=0.; i<ROCK_depth ; i++) {            // cascade of layers

    vec2 V =  U / vec2(RATIO,1);                  // cell shape
    height = 0.;

    vec2 u = V, iu = floor(u);
    #if <!VARIANT!>
    for( int k=0; k < 25; k++ ) {                 // loop on neighbord cells
        vec2 p = iu + vec2(k%5-2,k/5-2);          // searching for rock
    #else
    for( int k=0; k < 9; k++ ) {
        vec2 p = iu + vec2(k%3-1,k/3-1);
    #endif
        vec2 pc = CYCLE==vec2(0) ? p : mod(p,CYCLE);  // cyclicle version
        for( int n=0; n<ROCK_nb; n++) {
            vec2 o = <!hash3i!>(vec3(pc,n)).xy,
                    r = p - u + o;
            #if 0                                     // last level = peebles
            if (i==ROCK_depth-1.) {
                O =  max(O,.4*sqrt(max(0.,1.-dot(r,r)*2.))); continue;
            }
            #endif
            vec2 id = pc+o; //r+u;                    // rock location
            vec3 H =  <!hash3i!>(vec3(id*1e4,1));
            // noisify the rock shape
            V = r + ROCK_noise_amp * <!fbm22!>(id+r/ROCK_noise_scale) * ROCK_noise_scale;
            V *= <!rot!>(6.28*H.x);                      // random rotation
            V = abs(V);
            float h = clamp((ROCK_radiusA+ROCK_radiusD*(2.*H.y-1.)-max(V.x,V.y))/BEVEL, 0.,1.);
            H = -1.+2.*<!hash3i!>(vec3(id*1e4,3));       // rand grad variation
            vec2 g = gradA * <!rot!>(3.14*H.x) * pow(grad_randA,H.y);
            h *= ( .5 + dot(g,r));
            if (i==0. && h>height) id0 = o, dir = 180.*H.x, ICslope = length(g);
            if (BLEND == 0)  height += h ;
            else             height = max(height, h);
    }   }
    // modulates rock with user defined function
    if (i==0.) height *= 1.+<!func!>(( (U+1e3*id0) *vec2(ICslope,1) )*<!rot!>(-radians(dir)) );
    O += s * height;
    U *= 4.; s /= 4.; CYCLE *= 4.;
}

if (BLEND == 0) O /= float((1+ROCK_nb)/2); // in ADD mode, renormalize

outColor = vec3( id0.x, O, dir );

