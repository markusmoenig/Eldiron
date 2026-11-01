outName = "Brians Value Noise";
outCategory = "2D Noise";
ParameterGroup = "Brians Value Noise";
ParamSlider = "globalScale", "Global Scale", "1.0", "0.0", "10";
ParamVector2 = "localScale", "Local Scale", "1.0", "1.0", "-1000", "1000";
ParameterGroup = "About";
ParamDescription = "Good quality 2D value noise.";

// Based on https://github.com/BrianSharpe/Wombat/blob/master/Value2D.glsl

vec2 P = inUV / globalScale * 1./localScale * 10.;

vec2 Pi = floor(P);
vec2 Pf = P - Pi;

vec4 Pt = vec4( Pi.xy, Pi.xy + 1.0 );
Pt = Pt - floor(Pt * ( 1.0 / 71.0 )) * 71.0;
Pt += vec2( 26.0, 161.0 ).xyxy;
Pt *= Pt;
Pt = Pt.xzxz * Pt.yyww;
vec4 hash = fract( Pt * ( 1.0 / 951.135664 ) );

vec2 blend = Pf * Pf * Pf * (Pf * (Pf * 6.0 - 15.0) + 10.0);
vec4 blend2 = vec4( blend, vec2( 1.0 - blend ) );
outColor = vec3( dot( hash, blend2.zxzx * blend2.wwyy ) );
