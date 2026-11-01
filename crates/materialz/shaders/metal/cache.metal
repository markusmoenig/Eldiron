// Raster intermediates use premultiplied RGBA and explicit footprint mip selection.
float4 mz_cache_level(texture2d_array<float,access::read_write> images,float2 q,uint layer,uint level) {
    uint2 size=uint2(images.get_width(level),images.get_height(level));
    float2 p=q*float2(size)-0.5f;int2 lo=int2(floor(p));float2 f=fract(p);
    float4 v[4];for(uint i=0;i<4;i++){uint2 xy=uint2(clamp(lo+int2(i&1u,i>>1u),int2(0),int2(size)-1));v[i]=images.read(xy,layer,level);}
    return mix(mix(v[0],v[1],f.x),mix(v[2],v[3],f.x),f.y);
}
MzSample mz_cache_lookup(texture2d_array<float,access::read_write> images,constant float4& region,float2 uv,float aa,uint layer) {
    float2 q=(uv-region.xy)/region.zw;
    float pixel=max(abs(region.z)/float(images.get_width()),abs(region.w)/float(images.get_height()));
    float lod=clamp(log2(max(aa/max(pixel,0.000001f),1.0f)),0.0f,float(images.get_num_mip_levels()-1));
    uint lo=uint(floor(lod)),hi=min(lo+1,images.get_num_mip_levels()-1);float f=fract(lod);
    float4 c=mix(mz_cache_level(images,q,layer,lo),mz_cache_level(images,q,layer,hi),f);
    float d=mix(mz_cache_level(images,q,layer+1,lo).x,mz_cache_level(images,q,layer+1,hi).x,f);
    return {float4(c.rgb/max(c.a,0.000001f),c.a),d};
}
