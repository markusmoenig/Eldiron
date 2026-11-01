outName = "Waves";
outCategory = "Pattern";

ParameterGroup = "Position Settings";
ParamSlider = "scale", "Scale", "1.0", "0.0", "10";
ParameterGroup = "About";
ParamHtml = "<html><p>Returns the current 3D position.</p></html>";

vec2 uv = inPos.xz * scale;

float w = 0.0;
float sw = 0.0;
float iter = 0.0;
float ww = 1.0;
uv += inTime * 0.5;

for(int i=0;i<24;i++) {
    // w += ww * wave(uv * 0.06 , vec2(sin(iter), cos(iter)) * 10.0, 2.0 + iter * 0.08, 2.0 + iter * 3.0);

    // float wave(vec2 uv, vec2 emitter, float speed, float phase){
	    // float dst = distance(uv, emitter);
	    // return pow((0.5 + 0.2 * sin(dst * phase - iTime * speed)), 5.0);
    // }

    vec2 emitter = vec2(sin(iter), cos(iter)) * 10.0;
    float speed = 2.0 + iter * 0.08;
    float phase = 2.0 + iter * 3.0;

	float dst = distance( uv * 0.06, emitter );
    w += ww * pow((0.5 + 0.2 * sin(dst * phase - inTime * speed)), 5.0);

    sw += ww;
    ww = mix(ww, 0.0115, 0.4);
    iter += 2.39996;
}

const float depth = 2.1;
outColor = vec3( inPos.y - ( w / sw * depth - depth ) );

