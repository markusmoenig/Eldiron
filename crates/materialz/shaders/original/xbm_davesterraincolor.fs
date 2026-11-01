outName = "Daves Terrain Coloring";
outCategory = "2D fBm";
inTerminal = "0", "Noise", "vec3(0)";

ParameterGroup = "Turbulence";
ParamSlider = "scale", "Scale", "1.0", "0.1", "100.0";
ParamColor = "higherRockColor", "Higher Rock", "float4( 0.08 * 10,0.05 * 10,0.03 * 10, 1 )";
ParamColor = "lowerRockColor", "Lower Rock", "float4( 0.45,.30,0.15, 1 )";
ParamColor = "grassColor", "Grass", "float4( 0.3, 0.3, 0.1, 1 )";
ParameterGroup = "About";
ParamHtml = "<html><p>Computes a layered turbulence / fbm on the values of the input layer.</p></html>";

vec3 matPos = inPos * scale;
vec3 normal = inNormal;

vec3 mat = vec3( 0 );
// specular = .0;
// ambient = .1;
// vec3 dir = normalize(pos-cameraPos);

// vec3 matPos = pos * 2.0;// ... I had change scale halfway though, this lazy multiply allow me to keep the graphic scales I had

// float disSqrd = dis * dis;// Squaring it gives better distance scales.

float f = clamp(InputFunc( inPos, matPos.xz * 0.05, inNormal, inTime, inInput1, inInput2, inInput3 ).x, 0.0,1.0);//*10.8;
f += InputFunc( inPos, (matPos.xz*.1+normal.yz*1.08)*.85, inNormal, inTime, inInput1, inInput2, inInput3 ).x;
f *= .55;
vec3 m = mix(vec3(.63*f+.2, .7*f+.1, .7*f+.1), vec3(f*.43+.1, f*.3+.2, f*.35+.1), f*.65);
mat = m*vec3(f*m.x+.36, f*m.y+.30, f*m.z+.28);

// Should have used smoothstep to add colours, but left it using 'if' for sanity...
/*
if (normal.y < .5)
{
    float v = normal.y;
    float c = (.5-normal.y) * 4.0;
    c = clamp(c*c, 0.1, 1.0);
    f = Noise(vec2(matPos.x*.09, matPos.z*.095+matPos.yy*0.15));
    f += Noise(vec2(matPos.x*2.233, matPos.z*2.23))*0.5;
    mat = mix(mat, vec3(.4*f), c);
    specular+=.1;
}*/

if (normal.y < .5)
{
    float v = normal.y;
    float c = (.5-normal.y) * 4.0;
    c = clamp(c*c, 0.1, 1.0);
    f = InputFunc( inPos, vec2(matPos.x*.09, matPos.z*.095+matPos.yy*0.15), inNormal, inTime, inInput1, inInput2, inInput3 ).x;
    f += InputFunc( inPos, vec2(matPos.x*2.233, matPos.z*2.23), inNormal, inTime, inInput1, inInput2, inInput3 ).x*0.5;
    mat = mix(mat, vec3(.4*f),c);
}

// Grass. Use the normal to decide when to plonk grass down...

/*
if (matPos.y < 45.35 && normal.y > .65)
{

    m = vec3(Noise(matPos.xz*.023)*.5+.15, Noise(matPos.xz*.03)*.6+.25, 0.0);
    m *= (normal.y- 0.65)*.6;
    mat = mix(mat, m, clamp((normal.y-.65)*1.3 * (45.35-matPos.y)*0.1, 0.0, 1.0));
}*/

/*

if (matPos.y < 45.35 && normal.y > .65)
{
    m = vec3(InputFunc( inPos, matPos.xz*.023, inNormal, inTime, inInput1, inInput2, inInput3 ).x*.5+.15, InputFunc( inPos, matPos.xz*.03, inNormal, inTime, inInput1, inInput2, inInput3 ).x*.6+.25, 0.0 );
    m *= (normal.y- 0.65)*.6;
    mat = mix(mat, m, clamp((normal.y-.65)*1.3 * (45.35-matPos.y)*0.1, 0.0, 1.0));
}

*/

// ---- Snow

/*
if (matPos.y > 80.0 && normal.y > .42)
{
    float snow = clamp((matPos.y - 80.0 - Noise(matPos.xz * .1)*28.0) * 0.035, 0.0, 1.0);
    mat = mix(mat, vec3(.7,.7,.8), snow);
    specular += snow;
    ambient+=snow *.3;
}*/

/*
float snowLine = 0.8;

if (matPos.y > snowLine && normal.y > .42)
{
    float snow = clamp((matPos.y - snowLine - InputFunc( inPos, matPos.xz * .1, inNormal, inTime, inInput1, inInput2, inInput3 ).x*28.0) * 0.035, 0.0, 1.0);
    mat = mix(mat, vec3(.7,.7,.8), snow);
    // specular += snow;
    // ambient+=snow *.3;
}
*/

outColor = vec3( mat );
