outName = "Ashimas Perlin Noise";
outCategory = "2D Noise";
ParameterGroup = "Ashimas Perlin Noise";
ParamSlider = "globalScale", "Global Scale", "1.0", "0.0", "10";
ParamVector2 = "localScale", "Local Scale", "1.0", "1.0", "-1000", "1000";
ParamVector2 = "rep", "Repeat", "0.0", "0.0", "-1000", "1000";
ParameterGroup = "About";
ParamDescription = "Good quality 2D perlin noise.";

// Copyright by Stefan Gustavson. Distributed under the MIT license.
// https://github.com/ashima/webgl-noise/blob/master/src/classicnoise2D.glsl

vec2 P = inUV / globalScale * 1. / localScale * 10.;

vec4 Pi = floor(P.xyxy) + vec4(0.0, 0.0, 1.0, 1.0);
vec4 Pf = fract(P.xyxy) - vec4(0.0, 0.0, 1.0, 1.0);
Pi = mod(Pi, rep.xyxy);
Pi = Pi - floor(Pi * (1.0 / 289.0)) * 289.0;
vec4 ix = Pi.xzxz;
vec4 iy = Pi.yyww;
vec4 fx = Pf.xzxz;
vec4 fy = Pf.yyww;

vec4 perm1 = ((ix*34.0)+1.0)*ix;
vec4 perm2 = perm1 - floor(perm1 * (1.0 / 289.0)) * 289.0;
perm2 += iy;
vec4 perm3 = ((perm2*34.0)+1.0)*perm2;
vec4 i = perm3 - floor(perm3 * (1.0 / 289.0)) * 289.0;

vec4 gx = fract(i * (1.0 / 41.0)) * 2.0 - 1.0 ;
vec4 gy = abs(gx) - 0.5 ;
vec4 tx = floor(gx + 0.5);
gx = gx - tx;

vec2 g00 = vec2(gx.x,gy.x);
vec2 g10 = vec2(gx.y,gy.y);
vec2 g01 = vec2(gx.z,gy.z);
vec2 g11 = vec2(gx.w,gy.w);

vec4 norm =  1.79284291400159 - 0.85373472095314 * vec4(dot(g00, g00), dot(g01, g01), dot(g10, g10), dot(g11, g11));
g00 *= norm.x;
g01 *= norm.y;
g10 *= norm.z;
g11 *= norm.w;

float n00 = dot(g00, vec2(fx.x, fy.x));
float n10 = dot(g10, vec2(fx.y, fy.y));
float n01 = dot(g01, vec2(fx.z, fy.z));
float n11 = dot(g11, vec2(fx.w, fy.w));

vec2 fade_xy = Pf.xy*Pf.xy*Pf.xy*(Pf.xy*(Pf.xy*6.0-15.0)+10.0);
vec2 n_x = mix(vec2(n00, n01), vec2(n10, n11), fade_xy.x);
float n_xy = mix(n_x.x, n_x.y, fade_xy.y);

outColor = vec3( ( ( 2.3 * n_xy ) + 1. ) / 2. );
