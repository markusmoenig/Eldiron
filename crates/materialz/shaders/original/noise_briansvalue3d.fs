outName = "Brians Value Noise";
outCategory = "3D Noise";
ParameterGroup = "Brians Value Noise";
ParamSlider = "globalScale", "Global Scale", "1.0", "0.0", "10";
ParamVector3 = "localScale", "Local Scale", "1.0", "1.0", "1.0", "-1000", "1000";
ParameterGroup = "About";
ParamDescription = "Good quality 3D value noise.";

// Based on https://github.com/BrianSharpe/Wombat/blob/master/Value3D.glsl

vec3 P = inPos / globalScale * 1./localScale * 10.;

vec3 Pi = floor(P);
vec3 Pf = P - Pi;
vec3 Pf_min1 = Pf - 1.0;

Pi.xyz = Pi.xyz - floor(Pi.xyz * ( 1.0 / 69.0 )) * 69.0;
vec3 Pi_inc1 = step( Pi, vec3( 69.0 - 1.5 ) ) * ( Pi + 1.0 );

vec4 Pt = vec4( Pi.xy, Pi_inc1.xy ) + vec2( 50.0, 161.0 ).xyxy;
Pt *= Pt;
Pt = Pt.xzxz * Pt.yyww;
vec2 hash_mod = vec2( 1.0 / ( 635.298681 + vec2( Pi.z, Pi_inc1.z ) * 48.500388 ) );
vec4 hash_lowz = fract( Pt * hash_mod.xxxx );
vec4 hash_highz = fract( Pt * hash_mod.yyyy );

vec3 blend = Pf * Pf * Pf * (Pf * (Pf * 6.0 - 15.0) + 10.0);
vec4 res0 = mix( hash_lowz, hash_highz, blend.z );
vec4 blend2 = vec4( blend.xy, vec2( 1.0 - blend.xy ) );
outColor = vec3( dot( res0, blend2.zxzx * blend2.wwyy ) );
