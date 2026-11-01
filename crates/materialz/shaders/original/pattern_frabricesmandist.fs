outName = "Manhattan Voronoi Distance";
outCategory = "Pattern";

ParameterGroup = "Manhattan Voronoi Distance";
ParamSlider = "globalScale", "Global Scale", "1.0", "0.0", "10";
ParamVector2 = "localScale", "Local Scale", "1.0", "1.0", "-1000", "1000";
ParamSlider = "lineSize", "Line Size", "2.0", "0.0", "10";
ParamSlider = "seed", "Seed", "0.0", "0.0", "1000.0";
ParameterGroup = "About";
ParamDescription = "Generates a manhattan voronoi distance grid.";

// Copyright by Fabrice Neyret. Used with permission.
// https://www.shadertoy.com/view/MtlyR8

const int N = 3;
const float R = .5;

vec2 U = inUV / globalScale * 1./localScale * 2.0;

float m=1e9,m2, c=1e2, v,w;

for (int k=0; k<N*N; k++) {
	vec2 iU = floor(U)+.5,
			g = iU + vec2(k%N,k/N)-1.;

	float srnd = 2. * fract(sin( dot(g, vec2(12.9898, 78.233) ) ) * 43758.5453 ) - 1.0;

	vec2 p =  g+ srnd*R -U
				+.1*sin(seed+vec2(1.6,0)+3.14*srnd),
			q = p * mat2(1,-1,1,1)*.707;
	c = min(c,length(p));
	p = abs(p); v = max(p.x,p.y);
	if (v < m) m2 = m, m = v;
	else if (v < m2) m2 = v;
}

v = m2-m;

outColor = vec3( smoothstep(0., lineSize, 100.*v) );
