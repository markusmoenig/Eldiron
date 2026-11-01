outName = "Fabrices Gabor Noise";
outCategory = "2D Noise";

ParameterGroup = "Gabor Noise";
ParamSlider = "NB", "Number of Blobs", "10", "1", "200";
ParamSlider = "SIZE", "Size of Blobs", "5.", "0.001", "10.";
ParamSlider = "dist", "Distance", "-0.485", "-2.0", "2.";
ParamSlider = "freq", "Frequency", "10.", "0.", "100.";
ParameterGroup = "Scale";
ParamSlider = "globalScale", "Global Scale", "1.0", "0.0", "10";
ParamVector2 = "localScale", "Local Scale", "1.0", "1.0", "-1000", "1000";
ParameterGroup = "About";
ParamDescription = 'Generates random stripes with displacements called blobs. Often used to simulate dunes and similar.';

// Based on https://www.shadertoy.com/view/XsBGDc by Fabrice Neyret. Used with permission.

vec2 scaledUV = inUV / globalScale * 1./ localScale;
const uint k = 1103515245U;

vec3 col= vec3(0.);

for (int i=0; i<NB; i++)
{
  uvec3 x = uvec3( i, 0, 0 );
  x = ((x>>8U)^x.yzx)*k;
  x = ((x>>8U)^x.yzx)*k;
  x = ((x>>8U)^x.yzx)*k;
  float noise1 = (vec3(x)*(1.0/float(0xffffffffU))).x;

  x = uvec3( i, 1, 0 );
  x = ((x>>8U)^x.yzx)*k;
  x = ((x>>8U)^x.yzx)*k;
  x = ((x>>8U)^x.yzx)*k;
  float noise2 = (vec3(x)*(1.0/float(0xffffffffU))).x;

  x = uvec3( i, 2, 0 );
  x = ((x>>8U)^x.yzx)*k;
  x = ((x>>8U)^x.yzx)*k;
  x = ((x>>8U)^x.yzx)*k;
  float noise3 = (vec3(x)*(1.0/float(0xffffffffU))).x;

  x = uvec3( i, 3, 0 );
  x = ((x>>8U)^x.yzx)*k;
  x = ((x>>8U)^x.yzx)*k;
  x = ((x>>8U)^x.yzx)*k;
  float noise4 = (vec3(x)*(1.0/float(0xffffffffU))).x;

  vec2 pos = vec2( 1.5*noise1,noise2 );
  pos = scaledUV - pos;
  vec2 dir = (1.+ (dist))*vec2(noise3,noise4) - (dist);
  dir = freq * dir;

  float g = exp(-(pos.x*pos.x)/(SIZE*SIZE)) * exp(-(pos.y*pos.y)/(SIZE*SIZE));
  float s= .5*sin(dot(pos,dir) * 2. * 3.14159265358979-10. );
  col +=s * g;
}

outColor = col;

