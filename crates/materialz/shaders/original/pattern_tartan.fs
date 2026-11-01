outName = "Tartan";
outCategory = "Pattern";

ParameterGroup = "Tartan";
ParamSlider = "globalScale", "Scale", "1.0", "0.0", "10";
ParamSlider = "width", "Width", "4.0", "0.0", "100";
ParamSlider = "settCount", "Count", "100.0", "0.0", "1000";
ParamColor = "color1", "Color 1", "float4(0.2, 0.2, 0.2, 1.0)";
ParamColor = "color2", "Color 2", "float4(1.0, 0., 0., 1.0)";
ParamColor = "color3", "Color 3", "float4(0.8, 0.8, 0., 1.0)";
ParameterGroup = "About";
ParamDescription = "Generates a tartan like cloth pattern.";

// Inspired by https://www.shadertoy.com/view/lsfXR7

vec2 uv = inUV * 1000. / globalScale;
float x = uv.x;
float y = uv.y;
float xo;
float yo;
float xp;
float yp;

xp = floor(x/width);
yp = floor(y/width);

bool isWarp = false;
if (mod(xp - yp, 4.0) < 2.0) isWarp = !isWarp;

float m = 0.0;
float sett = 0.0;
if (isWarp) {
    m = fract(x/width);
    sett = fract(xp/settCount);
} else {
    m = fract(y/width);
    sett = fract(yp/settCount);
}

if (sett > 0.5) sett = 1.0- sett;

vec3 col = color1;
if ( sett < .2) col = color2;
else if (sett < .3) col = color3;

if (m > 0.5) m = 1.0 - m;

if (m < 0.1 ) outColor = vec3(1.0,1.0,1.0);
else if (m < 0.15 ) outColor = vec3(0.1,0.1,1.0);
else outColor = col.xyz;


