outName = "Wave Noise";
outCategory = "2D Noise";

ParameterGroup = "Wave Noise";
ParamSlider = "globalScale", "Global Scale", "1.0", "0.0", "10";
ParamVector2 = "localScale", "Local Scale", "1.0", "1.0", "-1000", "1000";
ParameterGroup = "About";
ParamDescription = "Wave like noise.";

// Based on https://www.shadertoy.com/view/lscyD7

const float EULER = 2.7182818284590452353602874;

vec2 xy = (inUV + vec2( 0.5 ) ) / globalScale * localScale;

// --- polarToXyz

xy *= vec2(2.0 *3.1415,  3.1415);
float z = cos(xy.y);
float x = cos(xy.x)*sin(xy.y);
float y= sin(xy.x)*sin(xy.y);
vec3 xyz = normalize(vec3(x,y,z));

// --- getwaves5d

vec4 position = vec4( xyz, ( .5 * 2.0 - 1.0) * 10.0);
float dragmult = 10.0 + .5 * 10.0;
float timeshift = inTime;

float iter = 0.0;
float phase = 6.0;
float speed = 2.0;
float weight = 1.0;
float w = 0.0;
float ws = 0.0;
float seedWaves = 0.0;

for(int i=0;i<20;i++) {
    vec4 p;
    {
        float x = fract(4768.1232345456 * sin(seedWaves));
        seedWaves += 1.0;
        float y = fract(4768.1232345456 * sin(seedWaves));
        seedWaves += 1.0;
        float z = fract(4768.1232345456 * sin(seedWaves));
        seedWaves += 1.0;
        float w = fract(4768.1232345456 * sin(seedWaves));
        seedWaves += 1.0;
        p = (vec4(x,y,z,w) * 2.0 - 1.0) * 30.0;
    }

    float dst = distance( position, p );
    float res = pow(EULER, sin(dst * phase - timeshift * speed)) / EULER;

    float t_timeshift = 0.006 + timeshift;
    float res2 = pow(EULER, sin(dst * phase - t_timeshift * speed)) / EULER;

    position -= normalize(position - p) * (res - res2) * weight * dragmult;
    w += res * weight;
    iter += 12.0;
    ws += weight;
    weight = mix(weight, 0.0, 0.2);
    phase *= 1.2;
    speed *= 1.02;
}

outColor = vec3(1.0) * (w / ws);
