outName = "Ashimas Tileable Simplex Noise";
outCategory = "2D Noise";
ParameterGroup = "Simplex Noise";
ParamSlider = "globalScale", "Global Scale", "1.0", "0.0", "10";
ParamVector2 = "localScale", "Local Scale", "1.0", "1.0", "-1000", "1000";
ParamVector2 = "per", "Repeat", "0.0", "0.0", "-1000", "1000";
ParamSlider = "rot", "Rotation", "0.0", "0.0", "10";
ParameterGroup = "About";
ParamHtml = '<html><p>Good quality 2D perlin noise. Based on <a href=https://github.com/BrianSharpe/Wombat/blob/master/Perlin2D.glsl>this</a> implementation by Stefan Gustavson.</p></html>`';

// Copyright by Stefan Gustavson. Distributed under the MIT license.
// https://github.com/ashima/webgl-noise/blob/master/src/psrdnoise2D.glsl

vec2 pos = inUV / globalScale * localScale;

// vec4 mod289(vec4 x)
// {
//   return x - floor(x * (1.0 / 289.0)) * 289.0;
// }

// vec4 permute(vec4 x)
// {
//   return mod289(((x*34.0)+1.0)*x);
// }

// Hashed 2-D gradients with an extra rotation.
// (The constant 0.0243902439 is 1/41)
// vec2 rgrad2(vec2 p, float rot) {
//   float u = permute(permute(p.x) + p.y) * 0.0243902439 + rot; // Rotate by shift
//   u = fract(u) * 6.28318530718; // 2*pi
//   return vec2(cos(u), sin(u));
// }

// Offset y slightly to hide some rare artifacts
pos.y += 0.001;
// Skew to hexagonal grid
vec2 uv = vec2(pos.x + pos.y*0.5, pos.y);

vec2 i0 = floor(uv);
vec2 f0 = fract(uv);
// Traversal order
vec2 i1 = (f0.x > f0.y) ? vec2(1.0, 0.0) : vec2(0.0, 1.0);

// Unskewed grid points in (x,y) space
vec2 p0 = vec2(i0.x - i0.y * 0.5, i0.y);
vec2 p1 = vec2(p0.x + i1.x - i1.y * 0.5, p0.y + i1.y);
vec2 p2 = vec2(p0.x + 0.5, p0.y + 1.0);

// Integer grid point indices in (u,v) space
i1 = i0 + i1;
vec2 i2 = i0 + vec2(1.0, 1.0);

// Vectors in unskewed (x,y) coordinates from
// each of the simplex corners to the evaluation point
vec2 d0 = pos - p0;
vec2 d1 = pos - p1;
vec2 d2 = pos - p2;

// Wrap i0, i1 and i2 to the desired period before gradient hashing:
// wrap points in (x,y), map to (u,v)
vec3 xw = mod(vec3(p0.x, p1.x, p2.x), per.x);
vec3 yw = mod(vec3(p0.y, p1.y, p2.y), per.y);
vec3 iuw = xw + 0.5 * yw;
vec3 ivw = yw;

// Create gradients from indices

vec2 g0_in = vec2(iuw.x, ivw.x);

float perm1 = ((g0_in.x*34.0)+1.0)*g0_in.y;
float perm2 = perm1 - floor(perm1 * (1.0 / 289.0)) * 289.0;
perm2 += g0_in.y;
float perm3 = ((perm2*34.0)+1.0)*perm2;
float g0_u = perm3 - floor(perm3 * (1.0 / 289.0)) * 289.0;
g0_u += rot;
g0_u = fract(g0_u) * 6.28318530718; // 2*pi
vec2 g0 = vec2(cos(g0_u), sin(g0_u));

vec2 g1 = g0;
vec2 g2 = g0;

// vec4 mod289(vec4 x)
// {
//   return x - floor(x * (1.0 / 289.0)) * 289.0;
// }

// vec4 permute(vec4 x)
// {
//   return mod289(((x*34.0)+1.0)*x);
// }

// Hashed 2-D gradients with an extra rotation.
// (The constant 0.0243902439 is 1/41)
// vec2 rgrad2(vec2 p, float rot) {
//   float u = permute(permute(p.x) + p.y) * 0.0243902439 + rot; // Rotate by shift
//   u = fract(u) * 6.28318530718; // 2*pi
//   return vec2(cos(u), sin(u));
// }



// vec2 g0 = rgrad2(vec2(iuw.x, ivw.x), rot);
// vec2 g1 = rgrad2(vec2(iuw.y, ivw.y), rot);
// vec2 g2 = rgrad2(vec2(iuw.z, ivw.z), rot);

// Gradients dot vectors to corresponding corners
// (The derivatives of this are simply the gradients)
vec3 w = vec3(dot(g0, d0), dot(g1, d1), dot(g2, d2));

// Radial weights from corners
// 0.8 is the square of 2/sqrt(5), the distance from
// a grid point to the nearest simplex boundary
vec3 t = 0.8 - vec3(dot(d0, d0), dot(d1, d1), dot(d2, d2));

// Set influence of each surflet to zero outside radius sqrt(0.8)
t = max(t, 0.0);

// Fourth power of t
vec3 t2 = t * t;
vec3 t4 = t2 * t2;

// Final noise value is:
// sum of ((radial weights) times (gradient dot vector from corner))
float n = dot(t4, w);

// Rescale to cover the range [-1,1] reasonably well
outColor = vec3( 11.0*n );


