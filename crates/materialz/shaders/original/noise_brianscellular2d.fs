outName = "Brians Cellular Noise";
outCategory = "2D Noise";
ParameterGroup = "Brians Cellular Noise";
ParamSlider = "globalScale", "Global Scale", "1.0", "0.0", "10";
ParamVector2 = "localScale", "Local Scale", "1.0", "1.0", "-1000", "1000";
ParameterGroup = "About";
ParamDescription = "Good quality 2D cellular noise.";

// Based on https://github.com/BrianSharpe/Wombat/blob/master/Cellular2D.glsl

vec2 P = inUV / globalScale * 1./ localScale * 10.;

vec2 Pi = floor(P);
vec2 Pf = P - Pi;

vec4 Pt = vec4( Pi.xy, Pi.xy + 1.0 );
Pt = Pt - floor(Pt * ( 1.0 / 71.0 )) * 71.0;
Pt += vec2( 26.0, 161.0 ).xyxy;
Pt *= Pt;
Pt = Pt.xzxz * Pt.yyww;
vec4 hash_x = fract( Pt * ( 1.0 / 951.135664 ) );
vec4 hash_y = fract( Pt * ( 1.0 / 642.949883 ) );

hash_x = hash_x * 2.0 - 1.0;
hash_y = hash_y * 2.0 - 1.0;
const float JITTER_WINDOW = 0.25;
hash_x = ( ( hash_x * hash_x * hash_x ) - sign( hash_x ) ) * JITTER_WINDOW + vec4( 0.0, 1.0, 0.0, 1.0 );
hash_y = ( ( hash_y * hash_y * hash_y ) - sign( hash_y ) ) * JITTER_WINDOW + vec4( 0.0, 0.0, 1.0, 1.0 );

vec4 dx = Pf.xxxx - hash_x;
vec4 dy = Pf.yyyy - hash_y;
vec4 d = dx * dx + dy * dy;
d.xy = min(d.xy, d.zw);

outColor = vec3( min(d.x, d.y) * ( 1.0 / 1.125 ) );
