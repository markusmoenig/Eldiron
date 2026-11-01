outName = "Tex Coord - Float";
outCategory = "System";

ParameterGroup = "Texture Coordinates Settings";
ParamVector2 = "scale", "Scale", "1.0", "1.0", "-1000", "1000";
ParamSlider = "rot", "Rotate", "0.0", "0.0", "360";
ParameterGroup = "About";
ParamHtml = "<html><p>Returns the current texture UV coordinates as a float, i.e. the average of the x and y values.</p></html>";

vec2 a = sin(vec2(1.5707963, 0) + radians(rot) );
mat2 r = mat2(a, -a.y, a.x);

vec2 U = inUV * r;

outColor = vec3( ( U.x * scale.x + U.y * scale.y ) / 2.0 );

