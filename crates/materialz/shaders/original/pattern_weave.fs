outName = "Weave";
outCategory = "Pattern";

ParameterGroup = "Weave Pattern";
ParamSlider = "scale", "Scale", "1.0", "0.01", "10.0";
ParamSlider = "xSize", "Pattern X-Size", "0.5", "0.0", "1.0";
ParamSlider = "ySize", "Pattern Y-Size", "0.5", "0.0", "1.0";
ParamColor = "color1", "Color #1", "float4(1.0, 1.0, 1.0, 1.0)";
ParamColor = "color2", "Color #2", "float4(0.0, 0.0, 0.0, 1.0)";
ParameterGroup = "About";
ParamDescription = "Generates a weave pattern. The pattern can be modified via the pattern size properties.";

// Based on a shader by Fabrice Neyret. https://www.shadertoy.com/view/4dSXWR. Used with permission.

vec2 p=inUV * inResolution * 1./scale * 1./2.0;
vec4 o=vec4( 1, 1, 1, 1);

vec2 uv0 = inUV * 1./scale * 1./2.0;
vec2 m = vec2( xSize, ySize );

vec2 uv = uv0*8., iuv=floor(uv);
float d = mod(iuv.x+iuv.y,2.), s = mod(iuv.y,2.);
uv = fract (uv); if (d==1.) uv.x = 1.-uv.x;
uv = uv + vec2(-uv.y,uv.x);

float q = sign(s-.5)*sign(d-.5),
    size0 = m.x+m.y*cos(.5*3.1415927*uv.y) *q,
        l = abs(uv.x)-size0,
        v = smoothstep(0.,.1,abs(l)),
        v0 = step(0.,l);

float size = m.x+m.y*cos(.5*3.1415927*uv.x),
        ofs = (1.-size)*q;
            l = (uv.y-1.)-ofs;
float   v1 = step(0.,l),
        d0 =  mod(s+v1,2.),
        d1 =  mod(s+d+v1,2.);
v0 = d1<1. ? v0 : 0.;
v = (d1<1. ? v : 1.)*smoothstep(0.,.1,abs(l));

float col = v0 *(cos(8.*31.4*uv0.x)*cos(8.*31.4*uv0.y))
        + (1.-v0)*( d1==1. ? cos(2.*31.4*( q>0. ? 2.-uv.y : uv.y )*m.x/size)
                            : cos(2.*31.4*(uv.x*m.x/size0)) );
o = vec4(col*v);

o.r = o.r * color1.r + (1.0-o.r) * color2.r;
o.g = o.g * color1.g + (1.0-o.g) * color2.g;
o.b = o.b * color1.b + (1.0-o.b) * color2.b;

outColor = o.xyz;
