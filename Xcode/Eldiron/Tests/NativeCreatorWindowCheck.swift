// Real AppKit windows, using the checked-in Creator controller/view/renderer.
import AppKit
import MetalKit

@MainActor
final class CheckCreatorController: GameViewController {
    override func loadView() { view = RMTKView(frame: NSRect(x: 0, y: 0, width: 1300, height: 700)) }
}
@main
@MainActor
struct NativeCreatorWindowCheck {
    static func state(_ handle: UInt64) -> [String: Any] {
        guard let text = eldiron_creator_state_v1(handle) else { fatalError("Missing Creator") }
        defer { rust_free_string(text) }
        return try! JSONSerialization.jsonObject(with: Data(String(cString: text).utf8)) as! [String: Any]
    }
    static func pump(_ controller: CheckCreatorController) {
        let until = Date().addingTimeInterval(0.4)
        repeat {
            controller.renderer.updateOnce()
            RunLoop.current.run(until: Date().addingTimeInterval(0.02))
        } while Date() < until
    }
    static func main() {
        setbuf(stdout, nil)
        _ = NSApplication.shared
        NSApp.setActivationPolicy(.accessory)
        NSApp.finishLaunching()
        let lifetimeOnly = CommandLine.arguments.contains("--lifetime-only")
        let first = CheckCreatorController()
        let second = CheckCreatorController()
        let a = NSWindow(contentViewController: first)
        let b = NSWindow(contentViewController: second)
        a.isReleasedWhenClosed = false
        b.isReleasedWhenClosed = false
        a.setFrameOrigin(NSPoint(x: 40, y: 80))
        b.setFrameOrigin(NSPoint(x: 100, y: 120))
        a.makeKeyAndOrderFront(nil)
        b.makeKeyAndOrderFront(nil)
        pump(first)
        pump(second)
        let firstHandle = first.renderer.handle
        let secondHandle = second.renderer.handle
        precondition(firstHandle != 0 && secondHandle != 0 && firstHandle != secondHandle)
        precondition(a.isVisible && b.isVisible)
        for handle in [firstHandle, secondHandle] {
            let value = state(handle)
            print("Creator window", handle, "presentations", value["presentations"]!, "root CPU", value["root_cpu_bytes"]!, "readback", value["readback_bytes"]!)
            if !lifetimeOnly { precondition((value["presentations"] as! NSNumber).uint64Value > 0) }
            precondition((value["root_cpu_bytes"] as! NSNumber).uint64Value == 0)
            precondition((value["readback_bytes"] as! NSNumber).uint64Value == 0)
        }
        b.performClose(nil)
        RunLoop.current.run(until: Date().addingTimeInterval(0.05))
        precondition(!b.isVisible && second.renderer.handle == 0,
                     "Closing a retained window must immediately retire its Rust owner")
        precondition(eldiron_creator_state_v1(secondHandle) == nil)
        precondition(eldiron_creator_document_info_v1(secondHandle) == nil)
        pump(first)
        precondition(a.isVisible && first.renderer.handle == firstHandle)
        precondition((state(firstHandle)["root_cpu_bytes"] as! NSNumber).uint64Value == 0)
        a.performClose(nil)
        RunLoop.current.run(until: Date().addingTimeInterval(0.05))
        precondition(!a.isVisible && first.renderer.handle == 0)
        precondition(eldiron_creator_state_v1(firstHandle) == nil)
        // Controllers/windows deliberately remain alive beyond their hosts.
        withExtendedLifetime((first, second, a, b)) {}
        print(lifetimeOnly
            ? "Creator window close ownership passed; physical presentation is not accepted in lifetime-only mode"
            : "Real Creator windows present independently and retire retained owners on close")
    }
}
