//
//  MetalStates.swift
//  Xcode2Rust
//
//  Created by Markus Moenig on 30/10/22.
//

import MetalKit

class MetalStates {
    
    enum States : Int {
        case DrawDisc, CopyTexture, DrawTexture, DrawBox, DrawBoxExt, DrawTextChar, DrawBackPattern, DrawTextureWhiteAlpha, DrawGrid
    }
    
    var defaultLibrary          : MTLLibrary!

    let pipelineStateDescriptor : MTLRenderPipelineDescriptor
    
    var states                  : [Int:MTLRenderPipelineState] = [:]
    
    var renderer                : Renderer
    
    init(_ renderer: Renderer)
    {
        self.renderer = renderer
        
        defaultLibrary = renderer.device.makeDefaultLibrary()
        
        let vertexFunction = defaultLibrary!.makeFunction( name: "m4mQuadVertexShader" )

        pipelineStateDescriptor = MTLRenderPipelineDescriptor()
        pipelineStateDescriptor.vertexFunction = vertexFunction
        //        pipelineStateDescriptor.fragmentFunction = fragmentFunction
        pipelineStateDescriptor.colorAttachments[0].pixelFormat = MTLPixelFormat.bgra8Unorm;
        
        pipelineStateDescriptor.colorAttachments[0].isBlendingEnabled = true
        pipelineStateDescriptor.colorAttachments[0].rgbBlendOperation = .add
        pipelineStateDescriptor.colorAttachments[0].alphaBlendOperation = .add
        pipelineStateDescriptor.colorAttachments[0].sourceRGBBlendFactor = .sourceAlpha
        pipelineStateDescriptor.colorAttachments[0].sourceAlphaBlendFactor = .sourceAlpha
        pipelineStateDescriptor.colorAttachments[0].destinationRGBBlendFactor = .oneMinusSourceAlpha
        pipelineStateDescriptor.colorAttachments[0].destinationAlphaBlendFactor = .oneMinusSourceAlpha
        
        states[States.DrawDisc.rawValue] = createQuadState(name: "m4mDiscDrawable")
        states[States.CopyTexture.rawValue] = createQuadState(name: "m4mCopyTextureDrawable")
        states[States.DrawTexture.rawValue] = createQuadState(name: "m4mTextureDrawable")
        states[States.DrawBox.rawValue] = createQuadState(name: "m4mBoxDrawable")
        states[States.DrawBoxExt.rawValue] = createQuadState(name: "m4mBoxDrawableExt")
        states[States.DrawTextChar.rawValue] = createQuadState(name: "m4mTextDrawable")
        states[States.DrawBackPattern.rawValue] = createQuadState(name: "m4mBoxPatternDrawable")
        states[States.DrawTextureWhiteAlpha.rawValue] = createQuadState(name: "m4mTextureDrawableWhiteAlpha")
        states[States.DrawGrid.rawValue] = createQuadState(name: "m4mGridDrawable")
    }
    
    /// Creates a quod state from an optional library and the function name
    func createQuadState( library: MTLLibrary? = nil, name: String ) -> MTLRenderPipelineState?
    {
        let function : MTLFunction?
            
        if library != nil {
            function = library!.makeFunction( name: name )
        } else {
            function = defaultLibrary!.makeFunction( name: name )
        }
        
        var renderPipelineState : MTLRenderPipelineState?
        
        do {
            //renderPipelineState = try device.makeComputePipelineState( function: function! )
            pipelineStateDescriptor.fragmentFunction = function
            renderPipelineState = try renderer.device.makeRenderPipelineState( descriptor: pipelineStateDescriptor )
        } catch {
            print( "computePipelineState failed" )
            return nil
        }
        
        return renderPipelineState
    }
    
    func getState(state: States) -> MTLRenderPipelineState
    {
        return states[state.rawValue]!
    }
}
