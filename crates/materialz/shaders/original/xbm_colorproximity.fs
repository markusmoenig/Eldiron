outName = "Color Proximity";
outCategory = "2D fBm";
inTerminal = "0", "Color", "vec3(0)";

ParameterGroup = "Color Proximity";
ParamColor = "color", "Color", "float4( 0.0, 0.0, 0.0, 1 )";
ParamSlider = "varianceValue", "Variance", "1.0", "0.0", "10.0";
ParamSlider = "strength", "Strength", "1.0", "0.0", "10.0";
ParamList = "component", "Component", "0", "x, y, z";
ParameterGroup = "About";
ParamHtml = "<html><p>Computes a layered turbulence / fbm on the values of the input layer.</p></html>";

#define COLOR_PROXIMITY_MAXDIST 10

float variance = varianceValue;
float dist = strength;

outColor = vec3( 0.0 );

for(int i = -COLOR_PROXIMITY_MAXDIST; i <= COLOR_PROXIMITY_MAXDIST; i++) {
    if(abs(float(i)) > dist) continue;
    for(int j = -COLOR_PROXIMITY_MAXDIST; j <= COLOR_PROXIMITY_MAXDIST; j++) {
        if(abs(float(j)) > dist || length(vec2(i, j)) > dist) continue;
        if(length(distance(color, vec3( InputFunc( inPos, inUV + vec2( i, j ) / inResolution.xy, inNormal, inTime, inInput1, inInput2, inInput3 ).component ) ) ) < variance) {
            //separation = vec2(iTime, 0);
            outColor = vec3( 1.0, length( vec2( dist, 0 ) ) / (dist / 2.), 0.0 );
            return outColor;
        }
    }
}



