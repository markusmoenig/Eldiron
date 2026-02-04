//
//  Renderer.swift
//  Xcode2Rust Shared
//
//  Created by Markus Moenig on 16/10/22.
//

import Metal
import MetalKit
import simd

class Renderer: NSObject, MTKViewDelegate {
    
    let view                                    : RMTKView
    var device                                  : MTLDevice!
    
    var texture                                 : Texture2D? = nil
    var metalStates                             : MetalStates!

    var viewportSize                            : vector_uint2
    
    var screenWidth                             : Float = 0
    var screenHeight                            : Float = 0
    
    var nearestSampler                          : MTLSamplerState!
    var linearSampler                           : MTLSamplerState!
    
    var scaleFactor                             : Float
    
    var cmdQueue                                : MTLCommandQueue? = nil
    var cmdBuffer                               : MTLCommandBuffer? = nil
    var scissorRect                             : MTLScissorRect? = nil
    
    var fps                                     : UInt32 = 0
    
    var anim_counter                            : UInt = 0
    var last_time                               : Double = 0
    var time_counter                            : Double = 0
    
    init(metalKitView: RMTKView) {
        self.view = metalKitView
        self.device = metalKitView.device
        
        #if os(OSX)
        scaleFactor = Float(NSScreen.main!.backingScaleFactor)
        #else
        scaleFactor = Float(UIScreen.main.scale)
        #endif
        
        var descriptor = MTLSamplerDescriptor()
        descriptor.minFilter = .nearest
        descriptor.magFilter = .nearest
        nearestSampler = device.makeSamplerState(descriptor: descriptor)
        
        descriptor = MTLSamplerDescriptor()
        descriptor.minFilter = .linear
        descriptor.magFilter = .linear
        linearSampler = device.makeSamplerState(descriptor: descriptor)
        
        viewportSize = vector_uint2( 0, 0 )
        
        view.platformInit()
        
        super.init()
        
        metalStates = MetalStates(self)
        checkTexture()
        checkFramerate()
    }
    
    @discardableResult func checkTexture() -> Bool
    {
        if texture == nil || texture!.texture.width != Int(view.frame.width) || texture!.texture.height != Int(view.frame.height) {
            
            if texture == nil {
                texture = Texture2D(self)
            } else {
                texture?.allocateTexture(width: Int(view.frame.width), height: Int(view.frame.height))
            }
            
            viewportSize.x = UInt32(texture!.width)
            viewportSize.y = UInt32(texture!.height)
            
            screenWidth = Float(texture!.width)
            screenHeight = Float(texture!.height)
                        
            scissorRect = MTLScissorRect(x: 0, y: 0, width: texture!.texture.width, height: texture!.texture.height)
    
            return true
        }
        return false
    }
    
    func draw(in view: MTKView) {
        
        rust_update()

        checkTexture()
        //print(screenWidth, screenHeight)
        guard let drawable = view.currentDrawable else {
            return
        }
        
        if last_time == 0 {
            last_time = Double(Date().timeIntervalSince1970)
        } else {
            let curr_time = Double(Date().timeIntervalSince1970)
            time_counter += (curr_time - last_time)

            if time_counter > Double(GAME_TICK_IN_MS) / 1000 {
                time_counter -= Double(GAME_TICK_IN_MS) / 1000
                anim_counter += 1
            }
            last_time = curr_time
        }
        
//        #if DEBUG
//        let startTime = Double(Date().timeIntervalSince1970)
//        #endif
                
        startDrawing()
         
        let count =  Int(texture!.width) *  Int(texture!.height) * 4
        let result = texture?.buffer?.contents().bindMemory(to: UInt8.self, capacity: count)
        rust_draw(result!, UInt32(texture!.width), UInt32(texture!.height), anim_counter)
        
        let renderPassDescriptor = view.currentRenderPassDescriptor
        renderPassDescriptor?.colorAttachments[0].loadAction = .clear
        let renderEncoder = cmdBuffer?.makeRenderCommandEncoder(descriptor: renderPassDescriptor!)
                
        drawTexture(renderEncoder: renderEncoder!)
        renderEncoder?.endEncoding()
        
        cmdBuffer?.present(drawable)
        
        stopDrawing()
        
//        #if DEBUG
        //print("Draw Time: ", (Double(Date().timeIntervalSince1970) - startTime) * 1000)
//        #endif
    }
    
    func startDrawing()
    {
        if cmdQueue == nil {
            cmdQueue = view.device!.makeCommandQueue()
        }
        cmdBuffer = cmdQueue!.makeCommandBuffer()
    }
    
    func stopDrawing(deleteQueue: Bool = false)
    {
        cmdBuffer?.commit()

        if deleteQueue {
            self.cmdQueue = nil
        }
        self.cmdBuffer = nil
    }
    
    /// Checks the framerate and applies it
    func checkFramerate() {
        let fps = rust_target_fps()
        
        if fps > 0 {
            view.enableSetNeedsDisplay = false
            view.isPaused = false
            view.preferredFramesPerSecond = Int(fps)
        } else {
            view.isPaused = true
            view.enableSetNeedsDisplay = true
        }
        self.fps = fps
    }
    
    /// Called after a user event function returns true
    func needsUpdate() {
        updateOnce()
        checkFramerate()
    }
    
    /// Updates the frame once
    func updateOnce() {
        #if os(OSX)
        let nsrect : NSRect = NSRect(x:0, y: 0, width: self.view.frame.width, height: self.view.frame.height)
        self.view.setNeedsDisplay(nsrect)
        #else
        self.view.setNeedsDisplay()
        #endif
    }
    
    func drawTexture(renderEncoder: MTLRenderCommandEncoder)
    {
        let width : Float = Float(texture!.width)
        let height: Float = Float(texture!.height)

        var settings = TextureUniform()
        settings.screenSize.x = screenWidth
        settings.screenSize.y = screenHeight
        settings.pos.x = 0
        settings.pos.y = 0
        settings.size.x = width * scaleFactor
        settings.size.y = height * scaleFactor
        settings.globalAlpha = 1
                
        let rect = MMRect( 0, 0, width, height, scale: scaleFactor )
        let vertexData = createVertexData(texture: texture!, rect: rect)
        
        renderEncoder.setVertexBytes(vertexData, length: vertexData.count * MemoryLayout<Float>.stride, index: 0)
        renderEncoder.setVertexBytes(&viewportSize, length: MemoryLayout<vector_uint2>.stride, index: 1)
        
        renderEncoder.setFragmentBytes(&settings, length: MemoryLayout<TextureUniform>.stride, index: 0)
        renderEncoder.setFragmentTexture(texture?.texture, index: 1)

        renderEncoder.setRenderPipelineState(metalStates.getState(state: .CopyTexture))
        renderEncoder.drawPrimitives(type: .triangle, vertexStart: 0, vertexCount: 6)
    }
    
    /// Creates vertex data for the given rectangle
    func createVertexData(texture: Texture2D, rect: MMRect) -> [Float]
    {
        let left: Float  = -texture.width / 2.0 + rect.x
        let right: Float = left + rect.width//self.width / 2 - x
        
        let top: Float = texture.height / 2.0 - rect.y
        let bottom: Float = top - rect.height

        let quadVertices: [Float] = [
            right, bottom, 1.0, 0.0,
            left, bottom, 0.0, 0.0,
            left, top, 0.0, 1.0,
            
            right, bottom, 1.0, 0.0,
            left, top, 0.0, 1.0,
            right, top, 1.0, 1.0,
        ]
        
        return quadVertices
    }
    
    func mtkView(_ view: MTKView, drawableSizeWillChange size: CGSize) {
    }
}
