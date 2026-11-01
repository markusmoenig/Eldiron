outName = "UV Position";
outCategory = "Material Mixer";
inMaterial = "0", "Material1", "Material";
inMaterial = "1", "Material2", "Material";
inMaterial = "2", "Material3", "Material";
inMaterial = "3", "Material4", "Material";

ParameterGroup = "UV Position Settings";
ParamList = "dir", "Direction", "0", "x, y";
ParamSlider = "level1", "Offset 1", "0.25", "0.0", "1.0";
ParamSlider = "level2", "Offset 2", "0.5", "0.0", "1.0";
ParamSlider = "level3", "Offset 3", "0.75", "0.0", "1.0";
ParameterGroup = "About";
ParamDescription = "Mixes up to four materials in the given UV direction using the specified offsets between 0..1.";

vec2 uv = inUV + vec2( 0.5 );

if ( uv.dir <= level1 ) {
    #define Material1;
    outMaterial = Material1;
} else if ( uv.dir <= level2 ) {
    #define Material2;
    outMaterial = Material2;
} else if ( uv.dir <= level3 ) {
    #define Material3;
    outMaterial = Material3;
} else {
    #define Material4;
    outMaterial = Material4;
}

