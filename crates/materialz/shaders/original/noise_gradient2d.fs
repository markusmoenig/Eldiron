outName = "Gradient Noise";
outCategory = "2D Noise";
inTerminal = "0", "Hash", "vec3(0)";
ParameterGroup = "Gradient Noise";
ParamSlider = "globalScale", "Global Scale", "1.0", "0.0", "10";
ParamVector2 = "localScale", "Local Scale", "1.0", "1.0", "-1000", "1000";
ParameterGroup = "About";
ParamDescription = "Best quality 2D gradient noise.";

// Source code provided by Shane (https://www.shadertoy.com/user/Shane), used with permission.

vec2 f = inUV / globalScale * 1./localScale * 10.;
const vec2 e = vec2(0, 1);
vec2 p = floor(f), _p = p;
f -= p;

p = _p + e.xx;
float n = sin(dot(p, vec2(41, 289)));
p = fract(vec2(2097152, 262144)*n)*2. - 1.;
vec2 hash1 = cos(p*6.283 + inTime);

p = _p + e.yx;
n = sin(dot(p, vec2(41, 289)));
p = fract(vec2(2097152, 262144)*n)*2. - 1.;
vec2 hash2 = cos(p*6.283 + inTime);

p = _p + e.xy;
n = sin(dot(p, vec2(41, 289)));
p = fract(vec2(2097152, 262144)*n)*2. - 1.;
vec2 hash3 = cos(p*6.283 + inTime);

p = _p + e.yy;
n = sin(dot(p, vec2(41, 289)));
p = fract(vec2(2097152, 262144)*n)*2. - 1.;
vec2 hash4 = cos(p*6.283 + inTime);

vec2 w = f*f*f*(f*(f*6.-15.)+10.);
float c = mix(mix(dot(hash1, f - e.xx), dot(hash2, f - e.yx), w.x), mix(dot(hash3, f - e.xy), dot(hash4, f - e.yy), w.x), w.y);

outColor = vec3( c*.5 + .5 );
