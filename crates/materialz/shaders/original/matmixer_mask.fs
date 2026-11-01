outName = "Mask";
outCategory = "Material Mixer";
inMaterial = "0", "Material1", "Material";
inMaterial = "1", "Material2", "Material";
inMaterial = "2", "UtilityMask", "Utility";

ParameterGroup = "Mask Material Mixer";
ParamBoolean = "inMix", "Mix", "1";
ParamToolTip = "Performas a linear interpolates between the two materials.";
ParameterGroup = "About";
ParamDescription = "Selects the material based on the output of the Utility Mask. A value of 0 chooses Material1, 1 chooses Material2, values in between get interpolated (if Mix is on) or ignored.";

#define Material1;
#define Material2;
#define UtilityMask;

#define <!smartmix!>( v0,v1,vmean, mask )  vmean + ( mix( v0, v1, mask) - vmean ) / sqrt( 1.-2.*mask*(1.-mask) )

if ( UtilityMask == vec3(0) ) outMaterial = Material1;
else if ( UtilityMask == vec3(1) ) outMaterial = Material2;
else {
    if ( !inMix ) outMaterial.bump = -1.0;
    else {
        outMaterial.baseColor = mix( Material1.baseColor, Material2.baseColor, UtilityMask );
        // outMaterial.baseColor = <!smartmix!>( Material1.baseColor, Material2.baseColor, 0.5, UtilityMask );
        outMaterial.roughness = mix( Material1.roughness, Material2.roughness, UtilityMask.x );
        outMaterial.metallic = mix( Material1.metallic, Material2.metallic, UtilityMask.x );
        outMaterial.specular = mix( Material1.specular, Material2.specular, UtilityMask.x );
        outMaterial.bump = mix( Material1.bump, Material2.bump, UtilityMask.x );
    }
}

