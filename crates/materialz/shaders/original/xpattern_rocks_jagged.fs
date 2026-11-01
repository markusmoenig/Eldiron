outName = "Rocks: Jagged";
outCategory = "Pattern";

ParameterGroup = "Rock: Jagged Pattern";

ParamSlider = "globalScale", "Global", "1.", "0.0", "10.0";
ParamVector2 = "localScale", "Local", "1.", "1.", "-1000", "1000";

ParamBoolean = "createCracks", "Cracks", "0";
ParamBoolean = "extraRoughness", "Extra Roughness", "1";

ParameterGroup = "About";
ParamDescription = "Generates a rock pattern.";

<! FUNC rot2 !>
mat2 <! rot2 !>( float a ){ vec2 v = sin(vec2(1.570796, 0) + a);	return mat2(v, -v.y, v.x); }
<! FUNC END !>

<! FUNC hash21 !>
float <! hash21 !>(vec2 p, vec2 scale, float repScale) {
    p = mod(p*scale, repScale);
    return fract(sin(dot(p, vec2(57.927, 127.763)))*43758.5453);
}
<! FUNC END !>

#define RIGID
<! FUNC hash22 !>
vec2 <! hash22 !>(vec2 p, vec2 scale, float repScale) {

    p = mod(p*scale, repScale);
    float n = sin(dot(p, vec2(1, 113)));
    p = fract(vec2(2097152, 262144)*n);
    #ifdef RIGID
    return p;
    #else
    return cos(p*6.283 /*+ iGlobalTime*/)*.5 + .5;
    #endif

}
<! FUNC END !>

<! FUNC hash22G !>
vec2 <! hash22G !>(vec2 p, vec2 scale, float repScale) {

    p = mod(p*scale, repScale);
    float n = sin(dot(p, vec2(27, 57)));
    return fract(vec2(2097152, 262144)*n)*2. - 1.;

}
<! FUNC END !>

<! FUNC gradN2D !>
float <! gradN2D !>(in vec2 f, vec2 scale, float repScale){

   f *= repScale;

   const vec2 e = vec2(0, 1);
    vec2 p = floor(f);
    f -= p;
    vec2 w = f*f*(3. - 2.*f);

    float c = mix(mix(dot(<!hash22G!>(p + e.xx, scale, repScale), f - e.xx), dot(<!hash22G!>(p + e.yx, scale, repScale), f - e.yx), w.x),
                  mix(dot(<!hash22G!>(p + e.xy, scale, repScale), f - e.xy), dot(<!hash22G!>(p + e.yy, scale, repScale), f - e.yy), w.x), w.y);

    return c*.5 + .5;
}
<! FUNC END !>

<! FUNC fBm !>
float <!fBm!>(in vec2 p, vec2 scale, float repScale){

    p *= repScale;
    return <!gradN2D!>(p, scale, repScale)*.57 + <!gradN2D!>(p, scale, repScale*2.)*.28 + <!gradN2D!>(p, scale, repScale*4.)*.15;
}
<! FUNC END !>

<! FUNC smax !>
float <!smax!>(float a, float b, float k){

   float f = max(0., 1. - abs(b - a)/k);
   return max(a, b) + k*.25*f*f;
}
<! FUNC END !>

<! FUNC smin !>
float <!smin!>(float a, float b, float k){

   float f = max(0., 1. - abs(b - a)/k);
   return min(a, b) - k*.25*f*f;
}
<! FUNC END !>


<! FUNC rock !>
float <!rock!>(vec2 p){

    float taper = (p.y)*2.*.65 + .35;

    p = abs(p)*2.;

    float shape = max(p.x, p.y);
    return max(shape, taper);
}
<! FUNC END !>

<! FUNC splatter !>
float <!splatter!>(in vec2 p, float repScale, vec2 overlapFactor){

    p *= repScale;
    vec2 scale = vec2(.75, 1.5);

    vec2 pert = vec2(<!gradN2D!>(p, vec2(1), 8.), <!gradN2D!>(p + .5, vec2(1), 8.)) - .5;
    p += pert*.125;

    vec2 ip = floor(p/scale);
    p = mod(p, scale) - scale/2.;

    vec3 d = vec3(4);
    vec2 dScale = overlapFactor;

    for(int j=-3; j<=3; j++){
        for(int i=-3; i<=3; i++){
            vec2 id = vec2(i, j);
            vec2 q = id*scale - p;
            q *= 1. + (<!hash21!>(ip + id + 4., scale, repScale) - .5);
            q = <!rot2!>((<!hash21!>(ip + id, scale, repScale) - .5)*6.2831*.25 + 3.14159/2.)*q;
            q += (<!hash22!>(ip + id + 2., scale, repScale) - .5)*.5*dScale;
            d.z = <!rock!>(q/dScale);
			d.y = max(d.x, min(d.y, d.z));
            d.x = min(d.x, d.z);
        }
    }

    return 1. - d.x;
}
<! FUNC END !>

vec2 uv = inUV * 1./globalScale * 1./localScale * 0.5;
vec2 p = fract(uv);

float l1 = <!splatter!>(p - vec2(.5), 9., vec2(3., 1.5));
float l2 = <!splatter!>(p + vec2(.5), 12., vec2(3., 1.5));
float l3 = <!splatter!>(p, 15., vec2(3., 1.5));
float l4 = <!splatter!>(p + vec2(.5, .5), 64., vec2(1.5));

float df = 1.;
df = min(min(l1, l2), (l3));

if ( createCracks ) {
    float cracks = smoothstep(0., .1, min(l1, l4) - .25);
    df = mix(df, cracks, .03);
}

if ( extraRoughness )
    df = mix(df, <!fBm!>(uv, vec2(1, 1), 16.), .05);

outColor = clamp( vec3( (.5 - df) * 2.0 + 0.5), 0., 1.);

