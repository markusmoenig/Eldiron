// Run via tools/check_native_player_apple.py. This uses the actual Swift facade,
// Rust static library and detached CAMetalLayers; it does not open a window.
import Foundation
import QuartzCore

@main
struct NativePlayerHostCheck {
    @MainActor
    static func main() throws {
        #if os(macOS)
        checkNativeTextClient()
        #endif
        let source = try Data(contentsOf: URL(fileURLWithPath: CommandLine.arguments[1]))
        let document = SceneVMDocument(data: source)
        var layer: CAMetalLayer? = CAMetalLayer()
        let player = SceneVMHandle(layer: layer!, size: CGSize(width: 640, height: 480), scale: 1.5)!
        document.sceneVMHandle = player
        var errors: [String] = []
        player.onError = { errors.append($0) }
        // The Swift handle itself must keep the layer alive, independently of
        // the view/local variable that originally supplied it.
        layer = nil
        precondition(player.loadProject(data: source))
        for _ in 0..<5 { player.render(); Thread.sleep(forTimeInterval: 0.02) }
        precondition(errors.isEmpty, "Unexpected host errors: \(errors)")
        let state = player.state!
        precondition(state.presentations > 0 && state.frameCount > 0)
        precondition(state.readbackBytes == 0)
        precondition(player.saveProject() == source)
        let snapshot = try document.snapshot(contentType: .eldironProject)
        precondition(snapshot == source)
        let session = state.sessionId!
        player.resize(to: .zero, scale: 1.5)
        player.render()
        precondition(player.state!.presentations == state.presentations)
        precondition(player.state!.frameCount == state.frameCount)
        player.suspend()
        player.render()
        var replacement: CAMetalLayer? = CAMetalLayer()
        precondition(player.replaceLayer(replacement!, size: CGSize(width: 800, height: 600), scale: 1.5))
        replacement = nil
        player.resume()
        player.focus(true)
        player.render()
        precondition(player.state!.sessionId == session)
        precondition(player.state!.presentations > state.presentations)
        precondition(player.state!.readbackBytes == 0)
        precondition(player.saveProject() == source)
        let restoredSnapshot = try document.snapshot(contentType: .eldironProject)
        precondition(restoredSnapshot == source)
        document.sceneVMHandle = nil
        player.close()
        print("Swift/FFI player: actual presentations, zero implicit readback, byte-exact project source and view/layer reconstruction passed")
    }
}
