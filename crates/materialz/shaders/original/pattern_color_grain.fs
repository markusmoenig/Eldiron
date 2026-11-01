outName = "Tileable Color Grain";
outCategory = "Pattern";

ParameterGroup = "Color Grain";
ParamColor = "colorA", "Color A", "float4( 0, 0, 0, 1 )";
ParamSlider = "weightA", "Weight", "1.4", "0.0", "2.0";
ParamColor = "colorB", "Color B", "float4( 1, 1, 1, 1 )";
ParamSlider = "weightB", "Weight", "0.6", "0.0", "2.0";

ParameterGroup = "Scale", "true";
ParamSlider = "globalScale", "Global", "0.6", "0.0", "1.0";
ParamVector2 = "localScale", "Local", "1.", "1.", "-1000", "1000";
ParamSlider = "repeatScale", "Repeat", "16.", "0.0", "16.0";

ParameterGroup = "About";
ParamDescription = "Creates a grainy color pattern based on two weighted colors.";

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

vec2 uv = inUV;

vec3 col = mix(colorA.xyz, colorB.xyz, <!fBm!>(inUV*16.,globalScale * localScale, repeatScale));
col = mix(col*weightA, col*weightB, <!fBm!>(inUV*32. - .5, globalScale * localScale, repeatScale));

outColor = col;
