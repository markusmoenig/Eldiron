outName = "IQs Voronoi Noise";
outCategory = "2D Noise";

ParameterGroup = "IQs Voronoi Noise";
ParamSlider = "globalScale", "Global Scale", "1.0", "0.0", "10";
ParamVector2 = "localScale", "Local Scale", "1.0", "1.0", "-1000", "1000";
ParameterGroup = "About";
ParamDescription = "Good quality 2D voronoi noise. Returns cell id in x and distance in the y component. Use the Split Operator to access the individual components.";

// Copyright by Inigo Quilez. Distributed under the MIT license.
// https://www.shadertoy.com/view/MslGD8

vec2 x = inUV / globalScale * 1./localScale*10.;

vec2 n = floor( x );
vec2 f = fract( x );

vec3 m = vec3( 8.0 );
for( int j=-1; j<=1; j++ )
for( int i=-1; i<=1; i++ )
{
    vec2  g = vec2( float(i), float(j) );
    vec2  hash = n + g;

    hash=vec2(dot(hash,vec2(127.1,311.7)),dot(hash,vec2(269.5,183.3)));
    vec2 o = fract(sin(hash)*18.5453);

    vec2  r = g - f + o;
    float d = dot( r, r );
    if( d<m.x )
        m = vec3( d, o );
}

outColor = vec3( vec2( m.y+m.z, sqrt(m.x) ), 0.0 );


