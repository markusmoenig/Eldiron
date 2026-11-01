outName = "IQs Value Noise";
outCategory = "2D Noise";
inTerminal = "0", "Hash", "vec3(0)";
ParameterGroup = "IQs Value Noise";
ParamSlider = "globalScale", "Global Scale", "1.0", "0.0", "10";
ParamVector2 = "localScale", "Local Scale", "1.0", "1.0", "-1000", "1000";
ParameterGroup = "About";
ParamDescription = "Good quality 2D value noise.";

// Copyright by Inigo Quilez. Distributed under the MIT license.
// https://www.shadertoy.com/view/lsf3WH

vec2 x = inUV / globalScale * 1./localScale * 32.;
vec2 p = floor(x);
vec2 f = fract(x);
f = f*f*(3.0-2.0*f);

vec2 pHash = p;
pHash = 50.0*fract( pHash*0.3183099 + vec2(0.71,0.113));
float _pHash1 = -1.0+2.0*fract( pHash.x*pHash.y*(pHash.x+pHash.y) );

pHash = p + vec2( 1.0, 0.0);
pHash = 50.0*fract( pHash*0.3183099 + vec2(0.71,0.113));
float _pHash2 = -1.0+2.0*fract( pHash.x*pHash.y*(pHash.x+pHash.y) );

pHash = p + vec2( 0.0, 1.0);
pHash = 50.0*fract( pHash*0.3183099 + vec2(0.71,0.113));
float _pHash3 = -1.0+2.0*fract( pHash.x*pHash.y*(pHash.x+pHash.y) );

pHash = p + + vec2( 1.0, 1.0);
pHash = 50.0*fract( pHash*0.3183099 + vec2(0.71,0.113));
float _pHash4 = -1.0+2.0*fract( pHash.x*pHash.y*(pHash.x+pHash.y) );

float noise = mix(mix( _pHash1, _pHash2, f.x), mix( _pHash3, _pHash4, f.x),f.y);
noise = 0.5 + noise * 0.5;
outColor = vec3( noise );

