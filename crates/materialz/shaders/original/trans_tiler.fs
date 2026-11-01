outName = "Tiler";
outCategory = "Transform";
inTerminal = "0", "Shape", "vec3(0)";
inDefaultTerminal = "Disc";

ParameterGroup = "Warp";
ParamVector2 = "tiling", "Tile", "1.0", "1.0", "0", "10";
ParamList = "typeX", "Type X", "2", "Unchanged, Reversed, Triangle, Cos";
ParamList = "typeY", "Type Y", "2", "Unchanged, Reversed, Triangle, Cos";
ParamSlider = "globalScale", "Scale", "1.0", "0.0", "10";
ParamSlider = "rot", "Rotate", "0.0", "0.0", "360";
ParameterGroup = "About";
ParamDescription = "Repeats / tiles the input shape.";

// --- tilers / warper / profile functions ----------------

int Unchanged = 0, Reversed = 1, Triangle = 2, Cos = 3;

<! FUNC tiler !>
vec2 <!tiler!>(vec2 U, vec2 T) {
    return fract(U*T);
}
<! FUNC END !>

<! FUNC scale !>
vec2 <!scale!>(vec2 U, vec2 S) {
    return clamp( .5+(U-.5)/S, 0., 1. );
}
<! FUNC END !>

<! FUNC profile !>
vec2 <!profile!>(vec2 U, ivec2 type) {
    mat4x2 M = mat4x2( U, 1.-U, 2.*min(U,1.-U), .5-.5*cos(6.28319*U) );
    return vec2 ( M[type.x][0], M[type.y][1]);
}
<! FUNC END !>

<! FUNC polar !>
vec2 <!polar!>(vec2 U) {
    U = 2.*U-1.;
    return vec2(atan(U.y,U.x)/6.28319 + .5, length(U));
}
<! FUNC END !>

// --------------------------------------------------------

<! FUNC noise2 !>
vec2 <!noise2!>(vec2 p) {
    #define <!hash22!>(p)  fract( 18.5453 * sin( (p) * mat2(127.1,311.7,269.5,183.3)) )

    vec2 i = floor(p);
    vec2 f = fract(p); f = f*f*(3.-2.*f); // smoothstep
    vec2 v= mix( mix(<!hash22!>(i+vec2(0,0)),<!hash22!>(i+vec2(1,0)),f.x),
                  mix(<!hash22!>(i+vec2(0,1)),<!hash22!>(i+vec2(1,1)),f.x), f.y);
    return 2.*v-1.;
}
<! FUNC END !>

<! FUNC displace !>
vec2 <!displace!>(vec2 U, float amp, float scale) {
    return  U += amp/scale* <!noise2!>(U*scale);
}
<! FUNC END !>

<! FUNC fbm2 !>
vec2 <!fbm2!>(vec2 p ) { return <!noise2!>(p)/2. + <!noise2!>(2.*p)/4. + <!noise2!>(4.*p)/8.; }
<! FUNC END !>

vec2  TILES = vec2(5,.1);
ivec2 TYPE  = ivec2(1,0);  // 0:unchanged 1:reversed 2:triangle  3:cos()
float NOISE_A = .4,
      NOISE_S = 8.;

vec2 U = (inUV) / globalScale;

vec2 a = sin(vec2(1.5707963, 0) + radians(rot) );
mat2 r = mat2(a, -a.y, a.x);

U = U * r;

U = <!tiler!>(U, tiling );
U = <!profile!>(U,ivec2(typeX, typeY));

outColor = InputFunc( inPos, U, inNormal, inTime, inInput1, inInput2, inInput3 );

