//
//  Texture2D.swift
//  Xcode2Rust
//
//  Created by Markus Moenig on 30/10/22.
//

import MetalKit

class Texture2D                 : NSObject
{
    var texture                 : MTLTexture!
    
    var width                   : Float = 0
    var height                  : Float = 0
    
    var buffer                  : MTLBuffer? = nil
    
    var renderer                : Renderer!
    
    ///
    init(_ renderer: Renderer)
    {
        self.renderer = renderer
        
        super.init()
        allocateTexture(width: Int(renderer.view.frame.width), height: Int(renderer.view.frame.height))
    }
    
    init(_ renderer: Renderer, width: Int, height: Int)
    {
        self.renderer = renderer

        super.init()
        allocateTexture(width: width, height: height)
    }
    
    init(_ renderer: Renderer, texture: MTLTexture)
    {
        self.renderer = renderer
        self.texture = texture
        
        width = Float(texture.width)
        height = Float(texture.height)
        
        super.init()
    }
    
    deinit
    {
        print("release texture")
        texture = nil
    }
    
    func allocateTexture(width: Int, height: Int)
    {
        texture = nil
        buffer = nil
        
        let w = ((width + 15) / 16) * 16;
        
        let textureDescriptor = MTLTextureDescriptor()
        textureDescriptor.textureType = MTLTextureType.type2D
        textureDescriptor.pixelFormat = MTLPixelFormat.rgba8Unorm
        textureDescriptor.width = w == 0 ? 1 : w
        textureDescriptor.height = height == 0 ? 1 : height
        textureDescriptor.resourceOptions = []
        
        self.width = Float(w)
        self.height = Float(height)
        
        textureDescriptor.usage = MTLTextureUsage.unknown
                
        buffer = renderer.device.makeBuffer(length: w * height * 4)
        
        texture = buffer?.makeTexture(descriptor: textureDescriptor,
                                      offset: 0,
                                      bytesPerRow: w * 4)
    }
    
    func clear(_ clearColor: float4? = nil)
    {
        let color : SIMD4<Float>; if let v = clearColor { color = v } else { color = SIMD4<Float>(0,0,0,1) }
        
        let renderPassDescriptor = MTLRenderPassDescriptor()
        
        renderPassDescriptor.colorAttachments[0].clearColor = MTLClearColorMake(Double(color.x), Double(color.y), Double(color.z), Double(color.w))
        renderPassDescriptor.colorAttachments[0].texture = texture
        renderPassDescriptor.colorAttachments[0].loadAction = .clear
        let renderEncoder = renderer.cmdBuffer!.makeRenderCommandEncoder(descriptor: renderPassDescriptor)!
        renderEncoder.endEncoding()
    }
}
