outName = "Position";
outCategory = "System";

ParameterGroup = "Position Settings";
ParamVector3 = "scale", "Scale", "1.0", "1.0", "1.0", "-1000", "1000";
ParameterGroup = "About";
ParamHtml = "<html><p>Returns the current 3D position.</p></html>";

outColor = inPos * scale;

