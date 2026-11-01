inTerminal = "0", "Pos 3D", "vec3(pos)";
inTerminal = "1", "UV", "vec3(uv, 0.)";
inTerminal = "2", "Param", "vec3(0)";

outName = "Utility Node (2D Anti-Aliased)";
outCategory = "System";

ParameterGroup = "Utility Node (2D Anti-Aliased)";
ParamSlider = "level", "Quality Level", "4.0", "1.0", "10";
ParamVector2 = "scale", "Scale", "1.0", "1.0", "-1000", "1000";
ParameterGroup = "About";
ParamHtml = "<html><p>Connect the right output to an utility node to invoke it. Passes the 3D position, uv and parameter to the node and anti-aliases the result based on the quality level.</p></html>";

const float AA = level;
const int AA2 = int(AA*AA);

vec3 col = vec3( 0.0 );
vec2 pix = 2.0/inResolution.yy/AA;
for (int i=0; i<AA2; i++){

    float k = float(i);
    vec2 uvOffs = inUV + vec2(floor(k/AA), mod(k, AA)) * pix;
    col += InputFunc(inPos, uvOffs, inNormal, inTime, inInput1 );
}

col /= (AA*AA);
outColor = col;

