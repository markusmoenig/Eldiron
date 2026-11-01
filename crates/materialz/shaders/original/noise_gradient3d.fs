outName = "Gradient Noise";
outCategory = "3D Noise";
inTerminal = "0", "Hash", "vec3(0)";
ParameterGroup = "Gradient Noise";
ParamSlider = "globalScale", "Global Scale", "1.0", "0.0", "10";
ParamVector3 = "localScale", "Local Scale", "1.0", "1.0", "1.0", "-1000", "1000";
ParameterGroup = "About";
ParamDescription = "Best quality 3D gradient noise.";

// Source code provided by Shane (https://www.shadertoy.com/user/Shane), used with permission.

const vec2 e = vec2(0, 1);
vec3 f = inPos / globalScale * 1./localScale * 10.;
vec3 p = floor(f), _p = p;
f -= p;

vec3 w = f*f*(3. - 2.*f);
float time = 0.0;

p = _p + e.xxx;
p = vec3( dot(p,vec3(127.1, 311.7, 74.7)), dot(p,vec3(269.5, 183.3, 246.1)), dot(p,vec3(113.5, 271.9, 124.6)));
p = fract(sin(p)*43758.5453123)*2. - 1.;
vec2 a = sin(vec2(1.5707963, 0) + time*2.);
mat2 m = mat2(a, -a.y, a.x);
p.xy = m * p.xy;
vec3 hash1 = p;

p = _p + e.yxx;
p = vec3( dot(p,vec3(127.1, 311.7, 74.7)), dot(p,vec3(269.5, 183.3, 246.1)), dot(p,vec3(113.5, 271.9, 124.6)));
p = fract(sin(p)*43758.5453123)*2. - 1.;
p.xy = m * p.xy;
vec3 hash2 = p;

p = _p + e.xyx;
p = vec3( dot(p,vec3(127.1, 311.7, 74.7)), dot(p,vec3(269.5, 183.3, 246.1)), dot(p,vec3(113.5, 271.9, 124.6)));
p = fract(sin(p)*43758.5453123)*2. - 1.;
p.xy = m * p.xy;
vec3 hash3 = p;

p = _p + e.yyx;
p = vec3( dot(p,vec3(127.1, 311.7, 74.7)), dot(p,vec3(269.5, 183.3, 246.1)), dot(p,vec3(113.5, 271.9, 124.6)));
p = fract(sin(p)*43758.5453123)*2. - 1.;
p.xy = m * p.xy;
vec3 hash4 = p;

p = _p + e.xxy;
p = vec3( dot(p,vec3(127.1, 311.7, 74.7)), dot(p,vec3(269.5, 183.3, 246.1)), dot(p,vec3(113.5, 271.9, 124.6)));
p = fract(sin(p)*43758.5453123)*2. - 1.;
p.xy = m * p.xy;
vec3 hash5 = p;

p = _p + e.yxy;
p = vec3( dot(p,vec3(127.1, 311.7, 74.7)), dot(p,vec3(269.5, 183.3, 246.1)), dot(p,vec3(113.5, 271.9, 124.6)));
p = fract(sin(p)*43758.5453123)*2. - 1.;
p.xy = m * p.xy;
vec3 hash6 = p;

p = _p + e.xyy;
p = vec3( dot(p,vec3(127.1, 311.7, 74.7)), dot(p,vec3(269.5, 183.3, 246.1)), dot(p,vec3(113.5, 271.9, 124.6)));
p = fract(sin(p)*43758.5453123)*2. - 1.;
p.xy = m * p.xy;
vec3 hash7 = p;

p = _p + e.yyy;
p = vec3( dot(p,vec3(127.1, 311.7, 74.7)), dot(p,vec3(269.5, 183.3, 246.1)), dot(p,vec3(113.5, 271.9, 124.6)));
p = fract(sin(p)*43758.5453123)*2. - 1.;
p.xy = m * p.xy;
vec3 hash8 = p;

float c = mix(mix(mix(dot(hash1, f - e.xxx), dot(hash2, f - e.yxx), w.x),
                    mix(dot(hash3, f - e.xyx), dot(hash4, f - e.yyx), w.x), w.y),
                mix(mix(dot(hash5, f - e.xxy), dot(hash6, f - e.yxy), w.x),
                    mix(dot(hash7, f - e.xyy), dot(hash8, f - e.yyy), w.x), w.y), w.z);

outColor = vec3( c*.5 + .5 );

