outName = "Bump Height";
outCategory = "Material Mixer";
inMaterial = "0", "Material1", "Material";
inMaterial = "1", "Material2", "Material";

ParameterGroup = "About";
ParamDescription = "Selects the material based on the bump height.";

#define Material1;
#define Material2;

if ( Material1.bump >= Material2.bump ) outMaterial = Material1;
else outMaterial = Material2;

