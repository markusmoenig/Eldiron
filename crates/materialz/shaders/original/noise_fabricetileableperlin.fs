outName = "Fabrices Tileable Perlin Noise";
outCategory = "2D Noise";
ParameterGroup = "Fabrices Tileable Perlin";
ParamSlider = "scaleValue", "Scale", "1.0", "0.0", "10.0";
ParamToolTip = "The scale of the noise.";
ParamVector2 = "cycleValue", "Cycle", "0.5", "0.5", "0", "1";
ParamToolTip = "The repetition cycle of the noise.";
ParameterGroup = "About";
ParamDescription = "Best quality quality, tileable 2D perlin noise.";

<! FUNC hash3i !>
vec3 <!hash3i!>( uvec3 x )
{
    #define <!scramble!>  x = ( (x>>8U) ^ x.yzx ) * 1103515245U
    <!scramble!>; <!scramble!>; <!scramble!>;
    return vec3(x) / float(0xffffffffU);
}
<!FUNC END !>

<! FUNC grad !>
float <!grad!>(float r, vec2 p)
{
    #define <!unsigned!>(v) ( (v) >= 0. ? uint(v) : -1U-uint(-(v)) )
    #define <!hash!>(v) <!hash3i!>(uvec3(<!unsigned!>((v).x), <!unsigned!>((v).y),11)).x

    int h = int(r*256.) & 15;
    float u = h<8 ? p.x : p.y,
        v = h<4 ? p.y : h==12||h==14 ? p.x : 0.;
    return ((h&1) == 0 ? u : -u) + ((h&2) == 0 ? v : -v);
}
<!FUNC END !>

<! FUNC InoiseT2 !>
float <!InoiseT2!>(vec2 p, vec2 cycle)
{
    #define <!Gt!>(v) <!grad!>(<!hash!>(mod(I+v,cycle)),p-v)
    #define <!fade!>(t)  t * t * t * (t * (t * 6. - 15.) + 10.)

    vec2 I = floor(p); p -= I;
    vec2 f = <!fade!>(p);

    return mix( mix( <!Gt!>(vec2(0,0)),<!Gt!>(vec2(1,0)), f.x),
                mix( <!Gt!>(vec2(0,1)),<!Gt!>(vec2(1,1)), f.x), f.y );
}
<!FUNC END !>

float scale = inInput3.x == 0. ? scaleValue : inInput3.x;
vec2 CYCLE = inInput3.yz == vec2(0) ? cycleValue : inInput3.yz;

vec2 U = inUV;
float noise_S = 10. / scale;
U *= noise_S;

float noise = <!InoiseT2!>(U,CYCLE*noise_S);

outColor = vec3( (noise + 1.0 ) / 2.0 );

