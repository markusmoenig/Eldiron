outName = "Brians Perlin Noise";
outCategory = "2D Noise";
ParameterGroup = "Brians Perlin Noise";
ParamSlider = "globalScale", "Global Scale", "1.0", "0.0", "10";
ParamVector2 = "localScale", "Local Scale", "1.0", "1.0", "-1000", "1000";
ParameterGroup = "About";
ParamDescription = "Good quality 2D perlin noise.";

// Based on https://github.com/BrianSharpe/Wombat/blob/master/Perlin2D.glsl

vec2 P = inUV / globalScale * 1./ localScale * 10.;

vec2 Pi = floor(P);
vec4 Pf_Pfmin1 = P.xyxy - vec4( Pi, Pi + 1.0 );

vec4 Pt = vec4( Pi.xy, Pi.xy + 1.0 );
Pt = Pt - floor(Pt * ( 1.0 / 71.0 )) * 71.0;
Pt += vec2( 26.0, 161.0 ).xyxy;
Pt *= Pt;
Pt = Pt.xzxz * Pt.yyww;
vec4 hash_x = fract( Pt * ( 1.0 / 951.135664 ) );
vec4 hash_y = fract( Pt * ( 1.0 / 642.949883 ) );

vec4 grad_x = hash_x - 0.49999;
vec4 grad_y = hash_y - 0.49999;
vec4 grad_results = inversesqrt( grad_x * grad_x + grad_y * grad_y ) * ( grad_x * Pf_Pfmin1.xzxz + grad_y * Pf_Pfmin1.yyww );

grad_results *= 1.4142135623730950488016887242097;
vec2 blend = Pf_Pfmin1.xy * Pf_Pfmin1.xy * Pf_Pfmin1.xy * (Pf_Pfmin1.xy * (Pf_Pfmin1.xy * 6.0 - 15.0) + 10.0);
vec4 blend2 = vec4( blend, vec2( 1.0 - blend ) );
outColor = vec3( ( dot( grad_results, blend2.zxzx * blend2.wwyy ) + 1. ) / 2. );
