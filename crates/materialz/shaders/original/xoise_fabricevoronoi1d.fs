outName = "Fabrices Voronoi Noise";
outCategory = "2D Noise";

ParameterGroup = "Fabrices Voronoi Noise";
ParamSlider = "globalScale", "Global Scale", "1.0", "0.0", "10";
ParamVector2 = "localScale", "Local Scale", "1.0", "1.0", "-1000", "1000";
ParamVector2 = "cycle", "Cycle", "1.0", "1.0", "0", "1";
ParameterGroup = "About";
ParamDescription = "Good quality 3D voronoi noise. Returns cell id in the x component, closest cell in y, second closest in z. Use the Split Operator to access the individual components.";

<! FUNC hash !>
float <!hash!>( vec2 p ) { return fract(sin(dot(p,vec2(127.1,311.7)))*18.5453); }
<! FUNC END !>

<! FUNC voronoi !>
vec3 <!voronoi!>( vec2 u, vec2 CYCLE )
{
      #define <! disp !>(p) ( -(3.-1.)/2. + 3. * (p) )

      vec2  n = floor( u );
      float f = fract( u.x );

      vec2 m = vec2(1e5), left = vec2(-1e5),right = vec2(1e5), P;

      for( float g=-4.; g<=4.; g++ )
        {
            float o = <!hash!>(mod(n + vec2(g,0),CYCLE)),
                  d = g - f + <!disp!>(o);  // n+g+o - u
            if( abs(d) < m.x ) m = vec2( abs(d), o);
            if ( d < 0.) {          // seed on u left
                if (d>left.x) left.y = left.x, left.x = d;
                else if (d>left.y) left.y = d;
               }
            else         {          // seed on u right
                if (d<right.x) right.y = right.x, right.x = d;
                else if (d<right.y) right.y = d;
               }

        }
    // left and right brick ends
    P =  -m.x==left.x ? vec2( left.x+left.y,   left.x+right.x ) / 2.
                      : vec2( left.x+right.x, right.x+right.y ) / 2.;
    P += u.x;
    //  d to seed, hash, brick width, brick center.
    return vec3( P.y-P.x, (P.x+P.y)/2., m.x );
}
<! FUNC END !>

vec2 x = inUV / globalScale * 1./localScale*10.;
vec3 noise = <! voronoi !>( x, cycle );

outColor = noise;

