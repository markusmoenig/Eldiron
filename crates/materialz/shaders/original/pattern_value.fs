outName = "Value";
outCategory = "Pattern";

ParameterGroup = "Value Pattern";
ParamSlider = "globalValue", "Global", "0.0", "0.0", "10";
ParamVector3 = "localValue", "Local", "0.0", "0.0", "0.0", "-1000", "1000";
ParameterGroup = "About";
ParamDescription = "Generates a static value. The global value is added to the local value.";

outColor = globalValue + localValue;


