outName = "Wood";
outCategory = "2D Pattern";

ParameterGroup = "Wood Pattern";
ParamSlider = "scale", "Scale", "1.0", "0.0", "10";

ParamSlider = "veins_S", "Veins", "7.0", "0.0", "10";
ParamToolTip = "Number of veins.";

ParameterGroup = "Warp / Displacement";
ParamSlider = "warp_A", "Warp Amplitude", "0.05", "0.0", "1";
ParamToolTip = "Global displacement amplitude.";
ParamSlider = "warp_S", "Warp Scale", "1.0", "0.0", "3";
ParamToolTip = "Global displacement scale.";

ParameterGroup = "Noise", "false";
ParamSlider = "noise_A", "Noise Amplitude", "6.0", "0.0", "10";
ParamToolTip = "Noise amplitude.";
ParamVector2 = "noise_S", "Noise Scale", "0.2", "4.0", "0", "50";
ParamToolTip = "Noise scale.";

ParameterGroup = "Fibers", "false";
ParamSlider = "fibers_A", "Fibers Amplitude", "3.0", "0.0", "10";
ParamToolTip = "Fibers amplitude, fibers are dents in the knots.";
ParamSlider = "fibers_S", "Fibers Scale", "200.0", "0.0", "1000";
ParamToolTip = "Fibers scale, fibers are dents in the knots.";

ParameterGroup = "Knots", "false";
ParamSlider = "knot_A", "Strength", "0.6", "0.0", "4";
ParamToolTip = "Strength of the knots.";
ParamSlider = "inKnot_N", "Amount", "2", "0", "10";
ParamToolTip = "Number of knots.";
ParamVector2 = "knot_S", "Scale", "0.5", "3", "0", "50";
ParamToolTip = "Scale of knots.";
ParamVector2 = "knot_shape", "Shape", "0.7", "0.3", "0", "1";
ParamToolTip = "Shape of knots (Perlin).";

ParameterGroup = "About";
ParamDescription = "Generates a wood pattern.";

float  knot_N = float( inKnot_N );

<! FUNC hash31 !>
vec3 <!hash3i!>( uvec3 x )
{
    #define <!hash!>(v) <!hash3i!>(uvec3(v+float(0x0000ffffU),11)).x
    #define <!scramble!>  x = ( (x>>8U) ^ x.yzx ) * 1103515245U
    #define <!fade!>(t)  t * t * t * (t * (t * 6. - 15.) + 10.)

    <!scramble!>; <!scramble!>; <!scramble!>;
    return vec3(x) / float(0xffffffffU);
}
<! FUNC END !>

<! FUNC hash3f !>
vec3 <!hash3f!>(vec3 x) {
    return <!hash3i!>(uvec3( x * float(0xffffffffU) ) );
}
<! FUNC END !>

<! FUNC grad !>
float <!grad!>(float r, vec2 p) {
      int h = int(r*256.) & 15;
      float u = h<8 ? p.x : p.y,
            v = h<4 ? p.y : h==12||h==14 ? p.x : 0.;
      return ((h&1) == 0 ? u : -u) + ((h&2) == 0 ? v : -v);
}
<! FUNC END !>

<! FUNC Inoise2 !>
float <!Inoise2!>(vec2 p) {
    vec2 I = floor(p); p -= I;
    vec2 f = <!fade!>(p);

    #define <!G!>(v) <!grad!>(<!hash!>(I+v),p-v)

    return mix( mix( <!G!>(vec2(0,0)),<!G!>(vec2(1,0)), f.x),
                mix( <!G!>(vec2(0,1)),<!G!>(vec2(1,1)), f.x), f.y );
}
<! FUNC END !>

<! FUNC fbm2 !>
float <!fbm2!>(vec2 p) {
    float v = 0.,  a = .5;

    for (int i = 0; i < 3; i++, p*=2.,a/=2.)
        v += a * <!Inoise2!>(p);

    return v;
}
<! FUNC END !>

// --- version returning vec2

<! FUNC Inoise22 !>
vec2 <!Inoise22!>(vec2 p) {
    vec2 I = floor(p); p -= I;
    vec2 f = <!fade!>(p);

    #define <!G2!>(v) vec2( <!grad!>(<!hash!>(I+v),p-v), <!grad!>(<!hash!>(I+v+117.),p-v))

    return mix( mix( <!G2!>(vec2(0,0)),<!G2!>(vec2(1,0)), f.x),
                mix( <!G2!>(vec2(0,1)),<!G2!>(vec2(1,1)), f.x), f.y );
}
<! FUNC END !>

<! FUNC fbm22 !>
vec2 <!fbm22!>(vec2 p) {
    vec2 v = vec2(0); float  a = .5;

    for (int i = 0; i < 3; i++, p*=2.,a/=2.)
        v += a * <!Inoise22!>(p);

    return v;
}
<! FUNC END !>

// ---

<! FUNC bosoNoise1 !>
float <!bosoNoise1!>(float x) {
    float i = floor(x), f = fract(x);
    f = f*f*(3.-2.*f);
    return -1.+2.*mix( <!hash3i!>(uvec3(i,17,1)),<!hash3i!>(uvec3(i+1.,17,1)), f).x;
}
<! FUNC END !>

<! FUNC smoothfract !>
float <!smoothfract!>(float x, float s) {
    x = fract(x);
    return  x < s    ? smoothstep(s,-s, x)
          : x < 1.-s ? smoothstep(s,1.-s, x)
                     : smoothstep(1.+s,1.-s, x);
}
<! FUNC END !>

// ---

vec2 U = inUV / scale + inInput1.z * 12323.1211;
vec2 U0 = U;
vec3 O = vec3( 0 );

U.y += warp_A * <!fbm2!>(warp_S*U) / warp_S;
U.x += fibers_A * <!bosoNoise1!>(fibers_S*U.y) / fibers_S;
float v = noise_A * <!fbm2!>(noise_S*U) / noise_S.y,
        n = 0.;

#define <!knot!>(seed) smoothstep( knot_shape.x, knot_shape.y, abs(<!Inoise2!>(knot_S*U+seed)) )

for (float i=0.; i<knot_N; i++)
    n += <!knot!>(100.*<!hash3i!>(uvec3(i,23,2)).xy);

v += knot_A * n;
v = <!smoothfract!>(veins_S*v,.1);

outColor = vec3( v );


