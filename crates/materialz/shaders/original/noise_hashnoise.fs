outName = "Hash Noise";
outCategory = "2D Noise";
ParameterGroup = "About";
ParamDescription = 'Hash based noise.';

uvec3 x = uvec3( (inUV + vec2( 0.5 )) * inResolution, inTime );
const uint k = 1103515245U;

x = ((x>>8U)^x.yzx)*k;
x = ((x>>8U)^x.yzx)*k;
x = ((x>>8U)^x.yzx)*k;

outColor = vec3(x)*(1.0/float(0xffffffffU));
