// Compile once; compare one or two dispatches, optionally with parameter buffers.
import Foundation
import Metal

func fail(_ text: String) -> Never {
    FileHandle.standardError.write(Data((text + "\n").utf8))
    exit(1)
}
let args = CommandLine.arguments
guard [5, 6, 8].contains(args.count) else { fail("Expected source, reference, width, height, optional parameters and second reference/parameters") }
guard let device = MTLCreateSystemDefaultDevice() else { fail("No Metal device available") }
let source = try String(contentsOfFile: args[1], encoding: .utf8)
let width = Int(args[3])!
let height = Int(args[4])!
let options = MTLCompileOptions()
options.fastMathEnabled = false
let library = try device.makeLibrary(source: source, options: options)
guard let function = library.makeFunction(name: "material_preview") else { fail("Missing entry point") }
let pipeline = try device.makeComputePipelineState(function: function)
guard let queue = device.makeCommandQueue(),
      let output = device.makeBuffer(length: width * height * 16, options: .storageModeShared)
else { fail("Cannot allocate Metal resources") }

func compare(reference: String, parameters: String?) throws {
    let expected = try Data(contentsOf: URL(fileURLWithPath: reference))
    guard expected.count == width * height * 16 else { fail("Invalid reference length") }
    guard let command = queue.makeCommandBuffer(), let encoder = command.makeComputeCommandEncoder()
    else { fail("Cannot create command") }
    var parameterBuffer: MTLBuffer?
    if let parameters {
        let data = try Data(contentsOf: URL(fileURLWithPath: parameters))
        guard !data.isEmpty, data.count % 16 == 0 else { fail("Invalid parameter buffer length") }
        parameterBuffer = data.withUnsafeBytes { device.makeBuffer(bytes: $0.baseAddress!, length: data.count, options: .storageModeShared) }
        guard parameterBuffer != nil else { fail("Cannot allocate parameters") }
    }
    var dimensions = SIMD2<UInt32>(UInt32(width), UInt32(height))
    encoder.setComputePipelineState(pipeline)
    encoder.setBuffer(output, offset: 0, index: 0)
    encoder.setBytes(&dimensions, length: MemoryLayout.size(ofValue: dimensions), index: 1)
    if let parameterBuffer { encoder.setBuffer(parameterBuffer, offset: 0, index: 2) }
    encoder.dispatchThreads(MTLSize(width: width, height: height, depth: 1),
                           threadsPerThreadgroup: MTLSize(width: pipeline.threadExecutionWidth, height: 1, depth: 1))
    encoder.endEncoding()
    command.commit()
    command.waitUntilCompleted()
    if let error = command.error { fail("Metal execution failed: \(error)") }
    let actual = output.contents().bindMemory(to: Float.self, capacity: width * height * 4)
    var maximum: Float = 0
    expected.withUnsafeBytes { bytes in
        for i in 0..<(width * height * 4) {
            let bits = bytes.loadUnaligned(fromByteOffset: i * 4, as: UInt32.self)
            let reference = Float(bitPattern: UInt32(littleEndian: bits))
            let error = abs(actual[i] - reference)
            if !actual[i].isFinite || error > 0.0002 { fail("Mismatch at component \(i): CPU \(reference), Metal \(actual[i]), error \(error)") }
            maximum = max(maximum, error)
        }
    }
    print("Metal CPU agreement: \(width * height) pixels; maximum absolute error \(maximum)")
}
try compare(reference: args[2], parameters: args.count >= 6 ? args[5] : nil)
if args.count == 8 {
    try compare(reference: args[6], parameters: args[7])
    print("Both frames used the same compiled Metal pipeline.")
}
