outName = "Star";
outCategory = "Shape";

ParameterGroup = "Star Shape";
ParamSlider = "polyN", "Sides", "5", "3", "16";
ParamSlider = "starR", "Int. Radius", "0.5", "0", "1";
ParamSlider = "inBEVEL", "Bevel", "0.01", "0.0", "1";
ParamToolTip = "Beveling.";
ParamSlider = "ROTshape0", "Fixed Rot.", "0.0", "0.0", "180";
ParamToolTip = "Fixed rotation in degrees.";
ParamSlider = "ROTshape1", "Random Rot.", "0.0", "0.0", "180";
ParamToolTip = "Maximum random rotation in degrees.";
ParameterGroup = "Gradient", "false";
ParamBoolean = "GRAD", "Random Gradient", "0";
ParamToolTip = "Gives the shape a random gradient, useful for generating uneveness through visually tilting the shape.";
ParamSlider = "inStrengthGradient", "Gradient Strength", "0.5", "0.0", "1";
ParamToolTip = "Strength of the gradient.";

ParameterGroup = "Size & Offset", "false";
ParamSlider = "globalScale", "Global", "1.0", "0.0", "1.0";
ParamToolTip = "Global scale of the shape. If shape is used inside a pattern the scale should be 1. as the pattern will scale the shape automatically.";
ParamVector2 = "localScale", "Local", "1", "1", "0", "1";
ParamToolTip = "Local X/Y scale of the shape. If shape is used inside a pattern the scale should be 1. as the pattern will scale the shape automatically.";
ParamVector2 = "offset", "Offset", "0", "0", "0", "100";
ParamToolTip = "X/Y offset of the shape. If shape is used inside a pattern the offset should be 0. as the pattern will position the shape automatically.";

ParameterGroup = "About";
ParamDescription = "Generates a star shape.";

vec2  BEVEL = vec2(inBEVEL);

vec2  grad = vec2(1,0);
float grad_randA = 2.,
      grad_randD = 360.;

vec2 U = inUV / globalScale * localScale - offset;
vec2 Box = inInput1.xy == vec2( 0 ) ? vec2( 1, 1 ) : inInput1.xy;
float id = inInput1.z;
vec3 O = vec3( 0 );

uvec3 x = uvec3( vec3(id,.4,0) * float(0xffffffffU) );
const uint k = 1103515245U;
x = ((x>>8U)^x.yzx)*k;
x = ((x>>8U)^x.yzx)*k;
x = ((x>>8U)^x.yzx)*k;
vec3 rot = (vec3(x)*(1.0/float(0xffffffffU)));

float a = atan(U.x,-U.y) + radians(ROTshape0 + ROTshape1*(-1.+2.*rot.x)),
      l = length(U),
      b = 3.14159/float(polyN),
      tb = tan(b),
      s = .5*(starR * tb)/(sqrt(1.+tb*tb)-starR);
a = mod(a,2.*b)-b;
U = l * vec2(cos(a),sin(a)) / cos(b);
U.y = abs(U.y);
U.x -= .5*Box.y;

float A = -(s*U.x+U.y)/sqrt(1.+s*s),
      B = A * 2. / BEVEL.y,                 // distance in bevel
      m = B;

O += clamp( m ,0.,1.);

if ( GRAD ) {
      uvec3 x = uvec3( vec3(id,.3,0) * float(0xffffffffU) );
      const uint k = 1103515245U;
      x = ((x>>8U)^x.yzx)*k;
      x = ((x>>8U)^x.yzx)*k;
      x = ((x>>8U)^x.yzx)*k;
      vec3 rotHash = (vec3(x)*(1.0/float(0xffffffffU)));

      vec3 R = -1.+2.*rotHash;
      mat2 rot = mat2(sin(radians(grad_randD/2.*R.x) + vec4(1,0,2,1)*1.5707));

      grad *= rot * pow(grad_randA,R.y);
      O.rgb *= 3. - inStrengthGradient * 3. + dot(grad,U);
}

outColor = O;
