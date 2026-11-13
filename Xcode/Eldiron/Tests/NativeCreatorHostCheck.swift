// Link against the real Creator static library; main-thread foreign-layer check.
import AppKit
import QuartzCore

@main
struct NativeCreatorHostCheck {
    static func state() -> [String: Any] {
        guard let text = rust_host_state() else { fatalError("Missing Creator host") }
        defer { rust_free_string(text) }
        return try! JSONSerialization.jsonObject(with: Data(String(cString: text).utf8)) as! [String: Any]
    }
    static func pump(_ seconds: Double) {
        let until = Date().addingTimeInterval(seconds)
        repeat {
            precondition(rust_render() >= 0, "Creator presentation failed")
            if let requests = rust_take_requests() { rust_free_string(requests) }
            RunLoop.current.run(until: Date().addingTimeInterval(0.02))
        } while Date() < until
    }
    static func main() {
        _ = NSApplication.shared
        precondition(!rust_init(nil, 1200, 800, 1))
        let layer = CAMetalLayer()
        let pointer = Unmanaged.passUnretained(layer).toOpaque()
        precondition(rust_init(pointer, 1200, 800, 1))
        precondition(!rust_init(pointer, 1200, 800, 1), "Cannot replace an existing document by initializing twice")
        defer { rust_shutdown() }
        pump(0.5)
        var snapshot = state()
        precondition((snapshot["presentations"] as! NSNumber).uint64Value > 0)
        precondition((snapshot["root_cpu_bytes"] as! NSNumber).uint64Value == 0)
        precondition((snapshot["readback_bytes"] as! NSNumber).uint64Value == 0)
        let frames = (snapshot["presentations"] as! NSNumber).uint64Value
        rust_suspend()
        pump(0.1)
        precondition((state()["presentations"] as! NSNumber).uint64Value == frames)
        rust_resume()
        pump(0.1)
        rust_resize(1440, 960, 1.5)
        pump(0.1)
        snapshot = state()
        precondition((snapshot["logical_size"] as! [Double]) == [960, 640])
        let replacement = CAMetalLayer()
        precondition(rust_replace_layer(Unmanaged.passUnretained(replacement).toOpaque()))
        pump(0.1)
        precondition((state()["presentations"] as! NSNumber).uint64Value > frames)
        precondition((state()["readback_bytes"] as! NSNumber).uint64Value == 0)
        // Keep both backing layers alive through surface destruction.
        rust_shutdown()
        withExtendedLifetime((layer, replacement)) {}
        print("Native Creator CAMetalLayer presentation/lifecycle passed")
    }
}
