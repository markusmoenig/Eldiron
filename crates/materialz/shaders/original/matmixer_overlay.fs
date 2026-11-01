outName = "Overlay";
outCategory = "Material Mixer";
inMaterial = "0", "Base", "Material";
inMaterial = "1", "Overlay", "Material";

ParameterGroup = "Overlay Settings";
ParamSlider = "level", "Level", "0.0", "0.0", "1.0";
ParameterGroup = "About";
ParamDescription = "If the grayscale value of the base color of the overlay material is greater as level, choose the overlay, otherwise use the material.";

#define Overlay;
#define Base;

if ( ( Overlay.baseColor.x + Overlay.baseColor.y + Overlay.baseColor.z ) / 3. > level ) {
    outMaterial = Overlay;
    outMaterial.bump = Overlay.bump + Base.bump;
} else {
    outMaterial = Base;
}

