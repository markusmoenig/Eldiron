outName = "IQs Value Noise";
outCategory = "3D Noise";
inTerminal = "0", "Hash", "vec3(0)";
ParameterGroup = "IQs Value Noise";
ParamSlider = "globalScale", "Global Scale", "1.0", "0.0", "10";
ParamVector3 = "localScale", "Local Scale", "1.0", "1.0", "1.0", "-1000", "1000";
ParameterGroup = "About";
ParamDescription = "Good quality 3D value noise.";

// Copyright by Inigo Quilez. Distributed under the MIT license.
// https://www.shadertoy.com/view/4sfGzS

vec3 x = inPos / globalScale * localScale * 16.;
vec3 p = floor(x);
vec3 f = fract(x);
f = f*f*(3.0-2.0*f);

vec3 pHash = p;
pHash = fract( pHash*0.3183099+.1 );
pHash *= 17.0;
float _pHash1 = fract( pHash.x*pHash.y*pHash.z*(pHash.x+pHash.y+pHash.z));

pHash = p + vec3(1,0,0);
pHash = fract( pHash*0.3183099+.1 );
pHash *= 17.0;
float _pHash2 = fract( pHash.x*pHash.y*pHash.z*(pHash.x+pHash.y+pHash.z));

pHash = p + vec3(0,1,0);
pHash = fract( pHash*0.3183099+.1 );
pHash *= 17.0;
float _pHash3 = fract( pHash.x*pHash.y*pHash.z*(pHash.x+pHash.y+pHash.z));

pHash = p + vec3(1,1,0);
pHash = fract( pHash*0.3183099+.1 );
pHash *= 17.0;
float _pHash4 = fract( pHash.x*pHash.y*pHash.z*(pHash.x+pHash.y+pHash.z));

pHash = p + vec3(0,0,1);
pHash = fract( pHash*0.3183099+.1 );
pHash *= 17.0;
float _pHash5 = fract( pHash.x*pHash.y*pHash.z*(pHash.x+pHash.y+pHash.z));

pHash = p + vec3(1,0,1);
pHash = fract( pHash*0.3183099+.1 );
pHash *= 17.0;
float _pHash6 = fract( pHash.x*pHash.y*pHash.z*(pHash.x+pHash.y+pHash.z));

pHash = p + vec3(0,1,1);
pHash = fract( pHash*0.3183099+.1 );
pHash *= 17.0;
float _pHash7 = fract( pHash.x*pHash.y*pHash.z*(pHash.x+pHash.y+pHash.z));

pHash = p + vec3(1,1,1);
pHash = fract( pHash*0.3183099+.1 );
pHash *= 17.0;
float _pHash8 = fract( pHash.x*pHash.y*pHash.z*(pHash.x+pHash.y+pHash.z));

float noise = mix(mix(mix( _pHash1, _pHash2, f.x),
                   mix( _pHash3, _pHash4, f.x),f.y),
               mix(mix( _pHash5, _pHash6, f.x),
                   mix( _pHash7, _pHash8, f.x),f.y),f.z);

outColor = vec3( noise );
