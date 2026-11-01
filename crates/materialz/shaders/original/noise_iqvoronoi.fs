outName = "IQs Voronoi Noise";
outCategory = "3D Noise";

ParameterGroup = "IQs Voronoi Noise";
ParamSlider = "globalScale", "Global Scale", "1.0", "0.0", "10";
ParamVector3 = "localScale", "Local Scale", "1.0", "1.0", "1.0", "-1000", "1000";
ParameterGroup = "About";
ParamDescription = "Good quality 3D voronoi noise. Returns cell id in the x component, closest cell in y, second closest in z. Use the Split Operator to access the individual components.";

// Copyright by Inigo Quilez. Distributed under the MIT license.
// https://www.shadertoy.com/view/ldl3Dl

vec3 x = inPos / globalScale * 1./localScale*10.;

vec3 p = floor( x );
vec3 f = fract( x );

float id = 0.0;
vec2 res = vec2( 100.0 );
for( int k=-1; k<=1; k++ )
for( int j=-1; j<=1; j++ )
for( int i=-1; i<=1; i++ )
{
    vec3 b = vec3( float(i), float(j), float(k) );

    vec3 hash = vec3( dot(p+b,vec3(127.1,311.7, 74.7)), dot(p+b,vec3(269.5,183.3,246.1)), dot(p+b,vec3(113.5,271.9,124.6)) );
    hash = vec3(fract(sin(hash)*43758.5453123));

    vec3 r = vec3( b ) - f + hash;
    float d = dot( r, r );

    if( d < res.x )
    {
        id = dot( p+b, vec3(1.0,57.0,113.0 ) );
        res = vec2( d, res.x );
    }
    else if( d < res.y )
    {
        res.y = d;
    }
}

outColor = vec3( abs(id), sqrt( res ) );

