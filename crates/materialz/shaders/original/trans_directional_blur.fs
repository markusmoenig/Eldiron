outName = "Directional Blur";
outCategory = "Transform";
inTerminal = "0", "Shape", "vec3(0)";
inDefaultTerminal = "Disc";

ParameterGroup = "Directional Blur";
ParamSlider = "globalScale", "Scale", "1.0", "0.0", "10";
ParamSlider = "BLUR_A", "Blur Amp.", "0.3", "0.0", "2";
ParamSlider = "NOISE_S", "Noise Scale.", "4.0", "0.0", "16";
ParamSlider = "NBsamples", "Samples", "32", "0", "64";
ParameterGroup = "About";
ParamDescription = "Creates a directional blur, optionally with displacement.";

<! FUNC scale !>
vec2 <!scale!>(vec2 U, vec2 S) {
    return clamp( .5+(U-.5)/S, 0., 1. );
}
<! FUNC END !>

<! FUNC noise2 !>
vec2 <!noise2!>(vec2 p) {
    #define <!hash22!>(p)  fract( 18.5453 * sin( (p) * mat2(127.1,311.7,269.5,183.3)) )
    vec2 i = floor(p);
    vec2 f = fract(p); f = f*f*(3.-2.*f);
    vec2 v= mix( mix(<!hash22!>(i+vec2(0,0)),<!hash22!>(i+vec2(1,0)),f.x),
                 mix(<!hash22!>(i+vec2(0,1)),<!hash22!>(i+vec2(1,1)),f.x), f.y);
    return 2.*v-1.;
}
<! FUNC END !>

<! FUNC fbm2 !>
vec2 <!fbm2!>(vec2 p )
{
    return <!noise2!>(p)/2. + <!noise2!>(2.*p)/4. + <!noise2!>(4.*p)/8.;
}
<! FUNC END !>

<! FUNC Noise2 !>
vec2 <!Noise2!>(vec2 x) {
    return (<!noise2!>(x)+<!noise2!>(x+11.5)) / 2.;
}
<! FUNC END !>

vec2 U = inUV * 1./globalScale;

float amp = BLUR_A;
float scale = NOISE_S;

vec4 O = vec4(0);
float n = amp/scale * <!Noise2!>(U*scale).x;
vec2 dU = vec2(dFdx(n),dFdy(n))/length(fwidth(U*scale));

for (float i=-.5; i<.5; i+= 1./float(NBsamples)) {
    vec2 u = U + i*dU;
    O += vec4( InputFunc( inPos, u, inNormal, inTime, inInput1, inInput2, inInput3 ).y );
}

O /= float(NBsamples);

outColor = vec3( O.x );

