//
//  Shaders.metal
//  Xcode2Rust Shared
//
//  Created by Markus Moenig on 16/10/22.
//

#include <metal_stdlib>
using namespace metal;

#import "Metal.h"

typedef struct
{
    float4 clipSpacePosition [[position]];
    float2 textureCoordinate;
} RasterizerData;

// Quad Vertex Function
vertex RasterizerData
m4mQuadVertexShader(uint vertexID [[ vertex_id ]],
             constant VertexUniform *vertexArray [[ buffer(0) ]],
             constant vector_uint2 *viewportSizePointer  [[ buffer(1) ]])
{
    RasterizerData out;
    
    float2 pixelSpacePosition = vertexArray[vertexID].position.xy;
    float2 viewportSize = float2(*viewportSizePointer);
    
    out.clipSpacePosition.xy = pixelSpacePosition / (viewportSize / 2.0);
    out.clipSpacePosition.z = 0.0;
    out.clipSpacePosition.w = 1.0;
    
    out.textureCoordinate = vertexArray[vertexID].textureCoordinate;
    return out;
}

// --- SDF utilities

float m4mFillMask(float dist)
{
    return clamp(-dist, 0.0, 1.0);
}

float m4mBorderMask(float dist, float width)
{
    dist += 1.0;
    return clamp(dist + width, 0.0, 1.0) - clamp(dist, 0.0, 1.0);
}

float2 m4mRotateCCW(float2 pos, float angle)
{
    float ca = cos(angle), sa = sin(angle);
    return pos * float2x2(ca, sa, -sa, ca);
}

float2 m4mRotateCCWPivot(float2 pos, float angle, float2 pivot)
{
    float ca = cos(angle), sa = sin(angle);
    return pivot + (pos-pivot) * float2x2(ca, sa, -sa, ca);
}

float2 m4mRotateCW(float2 pos, float angle)
{
    float ca = cos(angle), sa = sin(angle);
    return pos * float2x2(ca, -sa, sa, ca);
}

float2 m4mRotateCWPivot(float2 pos, float angle, float2 pivot)
{
    float ca = cos(angle), sa = sin(angle);
    return pivot + (pos-pivot) * float2x2(ca, -sa, sa, ca);
}

// --- Draw a disc (with an optional texture)
fragment float4 m4mDiscDrawable(RasterizerData in [[stage_in]],
                               constant DiscUniform *data [[ buffer(0) ]],
                               texture2d<float> inTexture [[ texture(1) ]] )
{
    float2 uv = in.textureCoordinate * float2( data->radius * 2 + data->borderSize, data->radius * 2 + data->borderSize);
    uv -= float2( data->radius + data->borderSize / 2 );
    
    float dist = length( uv ) - data->radius + data->onion;
    if (data->onion > 0.0)
        dist = abs(dist) - data->onion;
    
    const float mask = m4mFillMask( dist );
    float4 col = float4( data->fillColor.xyz, data->fillColor.w * mask);
    
    float borderMask = m4mBorderMask(dist, data->borderSize);
    float4 borderColor = data->borderColor;
    borderColor.w *= borderMask;
    col = mix( col, borderColor, borderMask );

    if (data->hasTexture == 1 && col.w > 0.0) {
        constexpr sampler textureSampler (mag_filter::linear,
                                          min_filter::linear);
        
        float2 uv = in.textureCoordinate;
        uv.y = 1 - uv.y;
        uv = m4mRotateCCWPivot(uv, data->rotation, 0.5);

        float4 sample = float4(inTexture.sample(textureSampler, uv));
        
        col.xyz = sample.xyz;
        col.w = col.w * sample.w;
    }
    
    return col;
}

// --- Draw a box (with an optional texture)
fragment float4 m4mBoxDrawable(RasterizerData in [[stage_in]],
                               constant BoxUniform *data [[ buffer(0) ]],
                               texture2d<float> inTexture [[ texture(1) ]] )
{
    float2 uv = in.textureCoordinate * ( data->size );
    uv -= float2( data->size / 2.0 );
    
    float2 d = abs( uv ) - data->size / 2 + data->onion + data->round;
    float dist = length(max(d,float2(0))) + min(max(d.x,d.y),0.0) - data->round;
    
    if (data->onion > 0.0)
        dist = abs(dist) - data->onion;
    
    const float mask = m4mFillMask( dist );
    float4 col = float4( data->fillColor.xyz, data->fillColor.w * mask);
    
    float borderMask = m4mBorderMask(dist, data->borderSize);
    float4 borderColor = data->borderColor;
    borderColor.w *= borderMask;
    col = mix( col, borderColor, borderMask );
    
    if (data->hasTexture == 1 && col.w > 0.0) {
        constexpr sampler textureSampler(filter::nearest);
                                          //address::clamp_to_edge);
                                          //border_color::transparent_black);
        
        float2 uv = in.textureCoordinate;
        uv.y = 1 - uv.y;
                
        if (data->mirrorX)
            uv.x = 1 - uv.x;
        
        uv = m4mRotateCCWPivot(uv, data->rotation, 0.5);

        float4 sample = float4(inTexture.sample(textureSampler, uv));
        
        col.xyz = sample.xyz;
        col.w = col.w * sample.w;
    }

    //float4 col = float4( data->fillColor.x, data->fillColor.y, data->fillColor.z, m4mFillMask( dist ) * data->fillColor.w );
    //float4 col = float4( data->fillColor.x, data->fillColor.y, data->fillColor.z, smoothstep(0.0, -0.1, dist) * data->fillColor.w );
    //col = mix( col, data->borderColor, m4mBorderMask( dist, data->borderSize ) );
    //col = mix( col, data->borderColor, 1.0-smoothstep(0.0, data->borderSize, abs(dist)) );
    return col;
}

// --- Draw a rotated box
fragment float4 m4mBoxDrawableExt(RasterizerData in [[stage_in]],
                               constant BoxUniform *data [[ buffer(0) ]],
                               texture2d<float> inTexture [[ texture(1) ]] )
{
    float2 uv = in.textureCoordinate * data->screenSize;
    uv.y = data->screenSize.y - uv.y;
    uv -= float2(data->size / 2.0);
    uv -= float2(data->pos.x, data->pos.y);

    uv = m4mRotateCCW(uv, data->rotation);
    
    float2 d = abs( uv ) - data->size / 2.0 + data->onion + data->round;// - data->borderSize;
    float dist = length(max(d,float2(0))) + min(max(d.x,d.y),0.0) - data->round;
    
    if (data->onion > 0.0)
        dist = abs(dist) - data->onion;

    const float mask = m4mFillMask( dist );//smoothstep(0.0, pixelSize, -dist);
    float4 col = float4( data->fillColor.xyz, data->fillColor.w * mask);
    
    const float borderMask = m4mBorderMask(dist, data->borderSize);
    float4 borderColor = data->borderColor;
    borderColor.w *= borderMask;
    col = mix( col, borderColor, borderMask );
    
    if (data->hasTexture == 1 && col.w > 0.0) {
        constexpr sampler textureSampler (mag_filter::nearest,
                                          min_filter::nearest);
        
        float2 uv = in.textureCoordinate;
        uv.y = 1 - uv.y;
        
        if (data->mirrorX)
            uv.x = 1 - uv.x;
        
        uv -= data->pos / data->screenSize;
        uv *= data->screenSize / data->size;
        
        uv = m4mRotateCCWPivot(uv, data->rotation, (data->size / 2.0) / data->screenSize * (data->screenSize / data->size));
        
        float4 sample = float4(inTexture.sample(textureSampler, uv));
        
        col.xyz = sample.xyz;
        col.w = col.w * sample.w;
    }

    return col;
}

// --- Draw a checker pattern
fragment float4 m4mBoxPatternDrawable(RasterizerData in [[stage_in]],
                               constant BoxUniform *data [[ buffer(0) ]] )
{
    float2 uv = in.textureCoordinate * data->screenSize;
    uv.y = data->screenSize.y - uv.y;
    
    float4 checkerColor1 = data->fillColor;
    float4 checkerColor2 = data->borderColor;
    
    float4 col = checkerColor1;
    
    float cWidth = data->checkerSize.x / 2;
    float cHeight = data->checkerSize.y / 2;
    
    if ( fmod( floor( uv.x / cWidth ), 2.0 ) == 0.0 ) {
        if ( fmod( floor( uv.y / cHeight ), 2.0 ) != 0.0 ) col=checkerColor2;
    } else {
        if ( fmod( floor( uv.y / cHeight ), 2.0 ) == 0.0 ) col=checkerColor2;
    }
    
    return float4( col.xyz, 1);
}

// --- Copy texture
fragment float4 m4mCopyTextureDrawable(RasterizerData in [[stage_in]],
                                constant TextureUniform *data [[ buffer(0) ]],
                                texture2d<half, access::read> inTexture [[ texture(1) ]])
{
    float2 uv = in.textureCoordinate * data->size;
    uv.y = data->size.y - uv.y;
    
    const half4 colorSample = inTexture.read(uint2(uv));
    float4 sample = float4( colorSample );

    sample.w *= data->globalAlpha;

    return float4(sample.x / sample.w, sample.y / sample.w, sample.z / sample.w, sample.w);
}

// --- Draw a textue
fragment float4 m4mTextureDrawable(RasterizerData in [[stage_in]],
                                constant TextureUniform *data [[ buffer(0) ]],
                                texture2d<half> inTexture [[ texture(1) ]],
                                sampler textureSampler [[sampler(2)]])
{
    float2 uv = in.textureCoordinate;
    uv.y = 1 - uv.y;
    
    if (data->mirrorX)
        uv.x = 1 - uv.x;
    
    uv.x *= data->size.x;
    uv.y *= data->size.y;

    uv.x += data->pos.x;
    uv.y += data->pos.y;
    
    float4 sample = float4(inTexture.sample(textureSampler, uv));
    sample.w *= data->globalAlpha;

    return sample;
}

// --- Draw a texture with a white alpha
fragment float4 m4mTextureDrawableWhiteAlpha(RasterizerData in [[stage_in]],
                                constant TextureUniform *data [[ buffer(0) ]],
                                texture2d<half> inTexture [[ texture(1) ]],
                                sampler textureSampler [[sampler(2)]])
{
    float2 uv = in.textureCoordinate;
    uv.y = 1 - uv.y;
    
    if (data->mirrorX)
        uv.x = 1 - uv.x;
    
    uv.x *= data->size.x;
    uv.y *= data->size.y;

    uv.x += data->pos.x;
    uv.y += data->pos.y;
    
    float4 sample = float4(inTexture.sample(textureSampler, uv));
    sample.w *= data->globalAlpha;

    sample = mix(float4(1,1,1,1), sample, sample.w);
    
    return sample;
}

float m4mMedian(float r, float g, float b) {
    return max(min(r, g), min(max(r, g), b));
}

// --- Text
fragment float4 m4mTextDrawable(RasterizerData in [[stage_in]],
                                constant TextUniform *data [[ buffer(0) ]],
                                texture2d<float> inTexture [[ texture(1) ]])
{
    constexpr sampler textureSampler (mag_filter::linear,
                                      min_filter::linear);
    
    float2 uv = in.textureCoordinate;
    uv.y = 1 - uv.y;

    uv /= data->atlasSize / data->fontSize;
    uv += data->fontPos / data->atlasSize;

    float4 sample = inTexture.sample(textureSampler, uv );
        
    float d = m4mMedian(sample.r, sample.g, sample.b) - 0.5;
    float w = clamp(d/fwidth(d) + 0.5, 0.0, 1.0);
    return float4( data->color.x, data->color.y, data->color.z, w * data->color.w );
}

// --- Grid
fragment float4 m4mGridDrawable(RasterizerData in [[stage_in]],
                                constant GridUniform *data [[ buffer(0) ]] )
{
    float2 uv = in.textureCoordinate * data->screenSize;
    uv.y = data->screenSize.y - uv.y;
    
    float size = data->gridSize / 2;
    float scale = data->scale;
//    float onion = 0.0001;

    float2 gv = fract(uv / (size * scale));
    
    float2 d = abs( gv ) - 0.95;// + onion;
    float dist = length(max(d,float2(0))) + min(max(d.x,d.y),0.0);

    //float2 d = abs( uv ) - data->size / 2 + data->onion + data->round;
    //float dist = length(max(d,float2(0))) + min(max(d.x,d.y),0.0) - data->round;
    
    //if (data->onion > 0.0)
    //dist = abs(dist) - onion;
    //float dd = m4mBorderMask(dist, 0.01);
    
    float4 color = mix(float4(0), float(1), smoothstep(-0.05, 0.05, dist));
    
    return color;
}
