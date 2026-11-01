// Native distance payload and styling; validity is determined by the IR.
struct MzSample { float4 color; float distance; };
MzSample mz_image(float3 color) { return {float4(color,1.0f),0.0f}; }
float mz_coverage(float d, float aa) {
    float t=clamp(0.5f+d/mz_nonzero(aa),0.0f,1.0f);
    return 1.0f-t*t*(3.0f-2.0f*t);
}
MzSample mz_shape(float d,float aa) { return {float4(1.0f,1.0f,1.0f,mz_coverage(d,aa)),d}; }
float mz_circle(float2 uv,float2 center,float radius) { return length(uv-center)-max(radius,0.000001f); }
float mz_rectangle(float2 uv,float2 center,float2 halfSize,float radius) {
    float2 size=max(halfSize,float2(0.000001f));
    float r=clamp(radius,0.0f,min(size.x,size.y));
    float2 q=abs(uv-center)-size+r;
    return length(max(q,0.0f))+min(max(q.x,q.y),0.0f)-r;
}
float mz_combine(float a,float b,int mode,float smoothness) {
    if (mode==1) return min(a,b);
    if (mode==2) return max(a,-b);
    if (mode==3) return max(a,b);
    if (mode==4) {
        float k=max(smoothness,0.000001f);
        float h=clamp(0.5f+0.5f*(b-a)/k,0.0f,1.0f);
        return b*(1.0f-h)+a*h-k*h*(1.0f-h);
    }
    return b;
}
float4 mz_over(float4 front,float4 back) {
    float a=front.a+back.a*(1.0f-front.a);
    if (a<=0.0f) return float4(0.0f);
    return float4((front.rgb*front.a+back.rgb*back.a*(1.0f-front.a))/a,a);
}
MzSample mz_color(MzSample input,float4 color) {
    input.color=float4(color.rgb,clamp(color.a,0.0f,1.0f)*clamp(input.color.a,0.0f,1.0f));
    return input;
}
constant float2 MZ_BORDER_DIRECTIONS[16] = {
    float2(1,0),float2(0.9238795f,0.38268343f),float2(0.70710677f,0.70710677f),float2(0.38268343f,0.9238795f),
    float2(0,1),float2(-0.38268343f,0.9238795f),float2(-0.70710677f,0.70710677f),float2(-0.9238795f,0.38268343f),
    float2(-1,0),float2(-0.9238795f,-0.38268343f),float2(-0.70710677f,-0.70710677f),float2(-0.38268343f,-0.9238795f),
    float2(0,-1),float2(0.38268343f,-0.9238795f),float2(0.70710677f,-0.70710677f),float2(0.9238795f,-0.38268343f)
};
MzSample mz_border(MzSample input,float4 color,float outer,float band) {
    if (outer>0.0f) {
        color.a=clamp(color.a,0.0f,1.0f)*band/outer;
        float4 back=input.color;
        back.a=clamp(back.a/outer,0.0f,1.0f);
        input.color=mz_over(color,back);
        input.color.a*=outer;
    }
    return input;
}
