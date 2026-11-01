outName = "Normal Based Mix";
outCategory = "Mixer";
inTerminal = "0", "x", "vec3(0)";
inTerminal = "1", "y", "vec3(1)";

ParameterGroup = "Normal Based Mixer Settings";
ParamList = "component", "Normal", "1", "x, y, z";
ParamSlider = "normalBorder", "Normal Border", "0.5", "0.0", "1.0";
ParamSlider = "scaleNormal", "Scale Normal", "1.0", "0.0", "50.0";
ParameterGroup = "About";
ParamDescription = "The output is the result of the interpolation of y with x based on the current normal value. Only values greater than the normal border property will be mixed.";

outColor = inNormal.component > normalBorder ? mix( inInput1, inInput2, clamp( vec3( ( inNormal.component - normalBorder ) * scaleNormal ), 0., 1. ) ) : inInput1;

