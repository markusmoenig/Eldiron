struct MzContext {float3 position;float3 a;float3 b;float3 c;float valid;float3 normal;float time;float3 cell;float3 group;float instance_valid;float3 origin;float3 axis_x;float3 axis_y;};
uint mz_loop_count(float count,uint maximum){return isnan(count)?0u:uint(clamp(ceil(count),0.0f,float(maximum)));}
MzContext mz_context(float3 p,float3 a,float3 b,float3 c,float3 n,float t){MzContext v={p,a,b,c,1.0f,n,t,float3(0),float3(0),0.0f,float3(0),float3(1,0,0),float3(0,1,0)};return v;}
MzContext mz_context(float3 p,float3 a,float3 b,float3 c){return mz_context(p,a,b,c,float3(0,0,1),0.0f);}
MzContext mz_with_uv(MzContext ctx,float2 uv){ctx.position.xy=uv;return ctx;}
// Mechanical ports of the bundled Material-Z sources. See shaders/PROVENANCE.md.
// Near-zero dynamic divisors use a shared signed epsilon on both backends.
float mz_radians(float degrees) { return degrees * 0.017453292519943295f; }
float mz_nonzero(float v) { return abs(v) < 0.000001f ? (v < 0.0f ? -0.000001f : 0.000001f) : v; }
float2 mz_nonzero(float2 v) { return float2(mz_nonzero(v.x), mz_nonzero(v.y)); }

float3 mz_perlin(float2 uv, float globalScale, float2 localScale) {
    float2 P = uv / mz_nonzero(globalScale) / mz_nonzero(localScale) * 10.0f;
    float2 Pi = floor(P);
    float4 Pf_Pfmin1 = P.xyxy - float4(Pi, Pi + 1.0f);
    float4 Pt = float4(Pi.xy, Pi.xy + 1.0f);
    Pt = Pt - floor(Pt * (1.0f / 71.0f)) * 71.0f;
    Pt += float2(26.0f, 161.0f).xyxy;
    Pt *= Pt;
    Pt = Pt.xzxz * Pt.yyww;
    float4 hash_x = fract(Pt * (1.0f / 951.135664f));
    float4 hash_y = fract(Pt * (1.0f / 642.949883f));
    float4 grad_x = hash_x - 0.49999f;
    float4 grad_y = hash_y - 0.49999f;
    float4 grad_results = (1.0f / sqrt(grad_x * grad_x + grad_y * grad_y)) * (grad_x * Pf_Pfmin1.xzxz + grad_y * Pf_Pfmin1.yyww);
    grad_results *= 1.4142135623730950488016887242097f;
    float2 blend = Pf_Pfmin1.xy * Pf_Pfmin1.xy * Pf_Pfmin1.xy * (Pf_Pfmin1.xy * (Pf_Pfmin1.xy * 6.0f - 15.0f) + 10.0f);
    float4 blend2 = float4(blend, 1.0f - blend);
    float4 g = grad_results * blend2.zxzx * blend2.wwyy;
    return float3(( ((g.x + g.y) + g.z) + g.w + 1.0f) / 2.0f);
}

float3 mz_disc(float2 uv, float bevel, bool gradient, float strength, float globalScale, float2 localScale, float2 offset) {
    float2 U = uv / mz_nonzero(globalScale) * localScale - offset;
    float O = clamp((0.5f - length(U)) * 2.0f / mz_nonzero(bevel), 0.0f, 1.0f);
    if (gradient) {
        uint3 x = uint3(0u, uint(0.3f * float(0xffffffffu)), 0u);
        const uint k = 1103515245u;
        x = ((x >> 8u) ^ x.yzx) * k;
        x = ((x >> 8u) ^ x.yzx) * k;
        x = ((x >> 8u) ^ x.yzx) * k;
        float3 R = -1.0f + 2.0f * (float3(x) * (1.0f / float(0xffffffffu)));
        float angle = mz_radians(180.0f * R.x);
        float2 grad = float2(sin(angle + 1.5707f), sin(angle + 2.0f * 1.5707f)) * pow(2.0f, R.y);
        O *= 3.0f - strength * 3.0f + grad.x * U.x + grad.y * U.y;
    }
    return float3(O);
}

float2 mz_transform(float2 uv, float globalScale, float2 localScale, float2 offset, float rot) {
    float2 U = (uv-offset) / mz_nonzero(globalScale) / mz_nonzero(localScale);
    float2 a = sin(float2(1.5707963f, 0.0f) + mz_radians(rot));
    return float2(U.x*a.x+U.y*a.y, -U.x*a.y+U.y*a.x);
}

// Tile-language vector-safe divisors and curated Disc primitive.
float3 mz_nonzero(float3 v) {return float3(mz_nonzero(v.x),mz_nonzero(v.y),mz_nonzero(v.z));}
float4 mz_nonzero(float4 v) {return float4(mz_nonzero(v.x),mz_nonzero(v.y),mz_nonzero(v.z),mz_nonzero(v.w));}
float mz_disc_scalar(float2 uv,float bevel,float gradient,float strength,float global,float2 local,float2 offset) {return mz_disc(uv,bevel,gradient!=0.0f,strength,global,local,offset).x;}

float mz_inversesqrt(float v) {return 1.0f / sqrt(v);}
float2 mz_inversesqrt(float2 v) {return 1.0f / sqrt(v);}
float2 mz_radians(float2 v) {return v * 0.017453292519943295f;}
float3 mz_inversesqrt(float3 v) {return 1.0f / sqrt(v);}
float3 mz_radians(float3 v) {return v * 0.017453292519943295f;}
float4 mz_inversesqrt(float4 v) {return 1.0f / sqrt(v);}
float4 mz_radians(float4 v) {return v * 0.017453292519943295f;}

uint mz_uint(float v){return v<0.0f ? 0u-uint(min(trunc(-v),4294967040.0f)) : uint(min(trunc(v),4294967040.0f));}
float3 mz_hash33(float3 v) {
    uint3 x=uint3(mz_uint(v.x),mz_uint(v.y),mz_uint(v.z));
    for (int i=0;i<3;i++) x=((x>>8u)^x.yzx)*1103515245u;
    return float3(x)*(1.0f/4294967295.0f);
}

float3 mz_disc_cell(float2 uv, float bevel, bool gradient, float strength, float globalScale, float2 localScale, float2 offset, float3 cell) {
    float2 U = uv / mz_nonzero(globalScale) * localScale - offset;
    float O = clamp(((all(cell.xy == float2(0)) ? 0.5f : cell.y*0.5f) - length(U)) * 2.0f / mz_nonzero(bevel), 0.0f, 1.0f);
    if (gradient) {
        uint3 x = uint3(uint(cell.z*float(0xffffffffu)), uint(0.3f * float(0xffffffffu)), 0u);
        const uint k = 1103515245u;
        x = ((x >> 8u) ^ x.yzx) * k;
        x = ((x >> 8u) ^ x.yzx) * k;
        x = ((x >> 8u) ^ x.yzx) * k;
        float3 R = -1.0f + 2.0f * (float3(x) * (1.0f / float(0xffffffffu)));
        float angle = mz_radians(180.0f * R.x);
        float2 grad = float2(sin(angle + 1.5707f), sin(angle + 2.0f * 1.5707f)) * pow(2.0f, R.y);
        O *= 3.0f - strength * 3.0f + grad.x * U.x + grad.y * U.y;
    }
    return float3(O);
}

float mz_disc_context(float2 uv,float bevel,float gradient,float strength,float global,float2 local,float2 offset,float3 cell){return mz_disc_cell(uv,bevel,gradient!=0.0f,strength,global,local,offset,cell).x;}

float mz_dot(float a,float b){return a*b;}
float mz_dot(float2 a,float2 b){return a.x*b.x+a.y*b.y;}
float mz_dot(float3 a,float3 b){return (a.x*b.x+a.y*b.y)+a.z*b.z;}
float mz_dot(float4 a,float4 b){return ((a.x*b.x+a.y*b.y)+a.z*b.z)+a.w*b.w;}
float mz_length(float a){return abs(a);}
float mz_length(float2 a){return precise::sqrt(mz_dot(a,a));}
float mz_length(float3 a){return precise::sqrt(mz_dot(a,a));}
float mz_length(float4 a){return precise::sqrt(mz_dot(a,a));}

// Canonical float32 math: preserve operation order across CPU and Metal.
float mz_sin(float x){
    float q=floor(x*0.15915494f+0.5f);
    float r=(x-q*6.28125f)-q*0.0019353072f;
    if(r>1.5707963267948966f)r=3.141592653589793f-r;
    else if(r< -1.5707963267948966f)r= -3.141592653589793f-r;
    float z=r*r;
    float p=1.6059044e-10f*z-2.5052108e-8f;
    p=p*z+2.7557319e-6f;
    p=p*z-1.984127e-4f;
    p=p*z+8.333334e-3f;
    p=p*z-1.6666667e-1f;
    return r+(r*z)*p;
}
float2 mz_sin(float2 x){return float2(mz_sin(x.x),mz_sin(x.y));}
float3 mz_sin(float3 x){return float3(mz_sin(x.x),mz_sin(x.y),mz_sin(x.z));}
float4 mz_sin(float4 x){return float4(mz_sin(x.x),mz_sin(x.y),mz_sin(x.z),mz_sin(x.w));}
float mz_cos(float x){return mz_sin(x+1.5707963267948966f);}
float2 mz_cos(float2 x){return mz_sin(x+1.5707963267948966f);}
float3 mz_cos(float3 x){return mz_sin(x+1.5707963267948966f);}
float4 mz_cos(float4 x){return mz_sin(x+1.5707963267948966f);}

MzContext mz_resample_context(MzContext ctx,float3 p,float3 a,float3 b,float3 c,float3 n,float t){ctx.position=p;ctx.a=a;ctx.b=b;ctx.c=c;ctx.valid=1;ctx.normal=n;ctx.time=t;return ctx;}
MzContext mz_scope(MzContext ctx,float2 uv,float3 cell,float3 group){ctx.position.xy=uv;ctx.cell=cell;ctx.group=group;ctx.instance_valid=1;return ctx;}

MzContext mz_frame(MzContext ctx,float3 origin,float3 x,float3 y) {
    float3 a=ctx.axis_x,b=ctx.axis_y;
    ctx.origin += a*origin.x+b*origin.y+float3(0,0,origin.z);
    ctx.axis_x=a*x.x+b*x.y+float3(0,0,x.z);ctx.axis_y=a*y.x+b*y.y+float3(0,0,y.z);return ctx;
}
MzContext mz_scope_frame(MzContext ctx,float2 uv,float3 key,float3 group,float3 origin,float3 x,float3 y) {return mz_scope(mz_frame(ctx,origin,x,y),uv,key,group);}

MzContext mz_transform_frame(MzContext ctx,float global,float2 local,float2 offset,float angle) {
    float a=angle*0.017453292519943295f;float c=mz_sin(1.5707963f+a),s=mz_sin(a);float2 size=global*local;
    return mz_frame(ctx,float3(offset,0),float3(c*size.x,s*size.y,0),float3(-s*size.x,c*size.y,0));
}
