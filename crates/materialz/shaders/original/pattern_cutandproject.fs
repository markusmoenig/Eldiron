outName = "Cut and Project";
outCategory = "Pattern";

ParameterGroup = "Cut and Project Pattern";
ParamSlider = "VCELLST", "Cells", "8", "1", "20";
ParamToolTip = "The vertical amount of cells.";

ParamSlider = "BEVEL", "Bevel", "0.06", "0.0", "1";
ParamToolTip = "Beveling.";

ParamSlider = "GAP", "Gap", "0.05", "0.0", "1";
ParamToolTip = "GAP between cells.";

ParamSlider = "CP_tans_adir", "Tiling #1", "0.0", "0.", "1.";
ParamSlider = "CP_tans_u2dir", "Tiling #2", "0.0", "0.", "1.";
ParamSlider = "CP_tans_v2dir", "Tiling #3", "0.0", "0.", "1.";

ParameterGroup = "About";
ParamDescription = "Generates a tileable pattern using the cut and projection method. This pattern is slow to render.";

float CELL = float(VCELLST);

//------------------------------------------------------------------------------------
// The aperiodic tiling
//------------------------------------------------------------------------------------
//the two vectors defining the cut and project plane
<! FUNC constants !>
const float <!udir!>[5] = float[5](0.632456, 0.19544, -0.511667, -0.511667, 0.19544);
const float <!vdir!>[5] = float[5](0., 0.601501, 0.371748, -0.371748, -0.601501);
//the 3 directions perpendicular to the cut and project plane
const float <!u2dir!>[5] = float[5](0.632456, -0.511667, 0.19544, 0.19544, -0.511667);
const float <!v2dir!>[5] = float[5](0., 0.371748, -0.601501, 0.601501, -0.371748);
const float <!adir!>[5] = float[5](0.447214, 0.447214, 0.447214, 0.447214, 0.447214);
//Cut and project plane origin
float <!CPO!>[5];

//results of tiling search
struct <!Data!>{
	float dist;
	ivec2 sides;
	vec2 posInTile;
};
<! FUNC END !>

<! FUNC init !>
void <!init!>( const float CP_tans_adir, const float CP_tans_u2dir, const float CP_tans_v2dir ) {
    for(int i=0; i<5; i++)
		<!CPO!>[i] = CP_tans_u2dir * <!u2dir!>[i] + CP_tans_v2dir * <!v2dir!>[i] + CP_tans_adir * <!adir!>[i];
}
<! FUNC END !>

<! FUNC P2E5 !>
float[5] <!P2E5!>(vec2 z){
	float p[5];
	for(int i=0; i<5; i++)
		p[i] = <!CPO!>[i] + z.x * <!udir!>[i] + z.y * <!vdir!>[i];
	return p;
}
<! FUNC END !>

<! FUNC getRoundAndOffest !>
void <!getRoundAndOffest!>(in float[5] p, out float[5] ip, out float[5] ofs){
	for(int i=0; i<5; i++){
		ip [i] = floor(p[i]+.5);
		ofs[i] = p[i] - ip[i];
	}
}
<! FUNC END !>

<! FUNC getOfsDir !>
float[5] <!getOfsDir!>(float[5] ofs){
	float dir[5];
	for(int i=0; i<5; i++){
		dir[i] = 2. * float(ofs[i] > 0.) - 1.;
	}
	return dir;
}
<! FUNC END !>

<! FUNC projectOfs !>
vec2 <!projectOfs!>(float[5] ofs){
   //dot products
	vec2 pofs = vec2(0);
	for(int i=0; i<5; i++){
		pofs.x += ofs[i] * <!udir!>[i];
		pofs.y += ofs[i] * <!vdir!>[i];
	}
	return pofs;
}
<! FUNC END !>

<! FUNC Dist22V2 !>
float <!Dist22V2!>(vec2 a, float f){
	vec2 p = abs(a-.5) - .5;
	return max(p.x,p.y)*f;
}
<! FUNC END !>

<! FUNC section !>
<!Data!> <!section!>(int i, int j, float[5] p, float[5] ip, vec2 s){
    //these should be precomputed
    mat2 m = mat2(vec2(<!udir!>[i],<!vdir!>[i]), vec2(<!udir!>[j],<!vdir!>[j]));
    float f = dot(m[0],m[1]); f = sqrt(dot(m[0],m[0]) - f*f / dot(m[1],m[1]));
    m = inverse(m);
    //check intersection with dual
    vec2 lhs = vec2(ip[i] - <!CPO!>[i], ip[j] - <!CPO!>[j]) + 0.5*s;
	vec2 z = lhs * m;

    float[5]  ofs, q;
	q = <!P2E5!>(z);
	//the intersection can be on a neighbouring tile!
	for(int k=0; k<5; k++){
		q[k] = floor(q[k]+.5);
		if(k==i)      ofs[k]=p[k] - (ip[k] + .5 * (s.x - 1.));
        else if(k==j) ofs[k]=p[k] - (ip[k] + .5 * (s.y - 1.));
		else          ofs[k]=p[k] - q[k];
	}

	vec2 pofs = <!projectOfs!>(ofs);

	//get the face corresponding to the intersected dual
    vec2 pit = (m * pofs);

	float dist   = <!Dist22V2!>(pit, f);
	<!Data!> d1 = <!Data!>(dist, ivec2(i,j), pit);
	return d1;
}
<! FUNC END !>

<! FUNC DE !>
<!Data!> <!DE!>(vec2 z)
{
	float[5] p = <!P2E5!>(z);

	float[5] ip, ofs,dir;
	<!getRoundAndOffest!>(p,ip,ofs);
    // dir is the preferred direction. Most of the tiling shows up thanks to it.
    // comment "#define NO_GLITCH" below to see the difference
    // Now faster thanks to dir and coherent branching :).
    // One may notice that when zooming out (a lot) it slows down.
    dir = <!getOfsDir!>(ofs);
// #define NO_GLITCH

	for(int i=0; i<4; i++)
		for(int j=i+1; j<5; j++)
		{
            <!Data!> d1 = <!section!>(i, j, p, ip, vec2(dir[i],dir[j]));
            if(d1.dist < 0.) return d1;
		}
#ifdef NO_GLITCH
    for(int i=0; i<4; i++)
		for(int j=i+1; j<5; j++)
		{
            <!Data!> d1 = <!section!>(i, j, p, ip, vec2(-dir[i],dir[j]));
            if(d1.dist < 0.) return d1;
		}

    for(int i=0; i<4; i++)
		for(int j=i+1; j<5; j++)
		{
            <!Data!> d1 = <!section!>(i, j, p, ip, vec2(dir[i],-dir[j]));
            if(d1.dist < 0.) return d1;
		}

    for(int i=0; i<4; i++)
		for(int j=i+1; j<5; j++)
		{
            <!Data!> d1 = <!section!>(i, j, p, ip, vec2(-dir[i],-dir[j]));
            if(d1.dist < 0.) return d1;
		}
#endif

	return <!Data!>(0., ivec2(0), vec2(0));
}
<! FUNC END !>

<! FUNC getFaceSurf !>
float <!getFaceSurf!>(int i, int j){
	vec2 u,v;
	u[0]=<!udir!>[i]; u[1]=<!vdir!>[i];
	v[0]=<!udir!>[j]; v[1]=<!vdir!>[j];
	return abs(u[0]*v[1]-u[1]*v[0]);
}
<! FUNC END !>

// ---

vec2 U = CELL * (inUV - .5);

<!init!>( CP_tans_adir, CP_tans_u2dir, CP_tans_v2dir );
<!Data!> data = <!DE!>(U);

float d = abs(data.dist), faceCol = <!getFaceSurf!>(data.sides.x, data.sides.y) *2.5;
d = clamp( ( d - GAP/2. ) / BEVEL , 0., 1.);
float id = d == 0. ? 0. : faceCol< .7 ? .0 : 1.;

outColor = vec3( id, d, 0. );


