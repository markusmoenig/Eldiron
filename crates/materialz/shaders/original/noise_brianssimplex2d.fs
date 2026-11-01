outName = "Brians Simplex Noise";
outCategory = "2D Noise";
ParameterGroup = "Brians Simplex Noise";
ParamSlider = "globalScale", "Global Scale", "1.0", "0.0", "10";
ParamVector2 = "localScale", "Local Scale", "1.0", "1.0", "-1000", "1000";
ParameterGroup = "About";
ParamDescription = "Good quality 2D simplex noise.";

// Based on https://github.com/BrianSharpe/Wombat/blob/master/SimplexPerlin2D.glsl

vec2 P = inUV / globalScale * 1./localScale * 10.;

const float SKEWFACTOR = 0.36602540378443864676372317075294;
const float UNSKEWFACTOR = 0.21132486540518711774542560974902;
const float SIMPLEX_TRI_HEIGHT = 0.70710678118654752440084436210485;
const vec3 SIMPLEX_POINTS = vec3( 1.0-UNSKEWFACTOR, -UNSKEWFACTOR, 1.0-2.0*UNSKEWFACTOR );

P *= SIMPLEX_TRI_HEIGHT;
vec2 Pi = floor( P + dot( P, vec2( SKEWFACTOR ) ) );

vec4 Pt = vec4( Pi.xy, Pi.xy + 1.0 );
Pt = Pt - floor(Pt * ( 1.0 / 71.0 )) * 71.0;
Pt += vec2( 26.0, 161.0 ).xyxy;
Pt *= Pt;
Pt = Pt.xzxz * Pt.yyww;
vec4 hash_x = fract( Pt * ( 1.0 / 951.135664 ) );
vec4 hash_y = fract( Pt * ( 1.0 / 642.949883 ) );

vec2 v0 = Pi - dot( Pi, vec2( UNSKEWFACTOR ) ) - P;
vec4 v1pos_v1hash = (v0.x < v0.y) ? vec4(SIMPLEX_POINTS.xy, hash_x.y, hash_y.y) : vec4(SIMPLEX_POINTS.yx, hash_x.z, hash_y.z);
vec4 v12 = vec4( v1pos_v1hash.xy, SIMPLEX_POINTS.zz ) + v0.xyxy;

vec3 grad_x = vec3( hash_x.x, v1pos_v1hash.z, hash_x.w ) - 0.49999;
vec3 grad_y = vec3( hash_y.x, v1pos_v1hash.w, hash_y.w ) - 0.49999;
vec3 grad_results = inversesqrt( grad_x * grad_x + grad_y * grad_y ) * ( grad_x * vec3( v0.x, v12.xz ) + grad_y * vec3( v0.y, v12.yw ) );

const float FINAL_NORMALIZATION = 99.204334582718712976990005025589;

vec3 m = vec3( v0.x, v12.xz ) * vec3( v0.x, v12.xz ) + vec3( v0.y, v12.yw ) * vec3( v0.y, v12.yw );
m = max(0.5 - m, 0.0);
m = m*m;

outColor = vec3( ( ( dot(m*m, grad_results) * FINAL_NORMALIZATION ) + 1. ) / 2. );
