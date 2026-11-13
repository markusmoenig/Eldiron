// Swift owns the view/layer and platform services. Rust owns WGPU presentation.
import MetalKit
import QuartzCore

@MainActor
final class Renderer: NSObject, MTKViewDelegate {
    unowned let view: RMTKView
    private var timer: Timer?
    private var observers: [NSObjectProtocol] = []
    private var ready = false
    private var failed = false
    private var hasChanges = false
    private(set) var nativeInputState: NativeTextInput?
    private var sourceSnapshot: NativeTextSnapshot?
    // Outlive Rust's surface, including while the NSView/UIView is torn down.
    private var backingLayer: CAMetalLayer?

    private var density: Float {
        #if os(macOS)
        return Float(view.window?.backingScaleFactor ?? NSScreen.main?.backingScaleFactor ?? 1)
        #else
        return Float(view.traitCollection.displayScale)
        #endif
    }

    init(metalKitView: RMTKView) {
        view = metalKitView
        super.init()
        // MTKView's drawable loop must never compete with WGPU.
        view.isPaused = true
        view.enableSetNeedsDisplay = false
        view.framebufferOnly = false
        view.platformInit()
        guard let layer = view.layer as? CAMetalLayer else { return }
        backingLayer = layer
        let size = physicalSize()
        ready = rust_init(Unmanaged.passUnretained(layer).toOpaque(), size.0, size.1, density)
        guard ready else { NSLog("Could not initialize native Creator WGPU surface"); return }
        observeLifecycle()
        let timer = Timer(timeInterval: 1.0 / 60.0, repeats: true) { [weak self] _ in MainActor.assumeIsolated { self?.tick() } }
        self.timer = timer
        RunLoop.main.add(timer, forMode: .common)
    }

    deinit {
        timer?.invalidate()
        observers.forEach { NotificationCenter.default.removeObserver($0) }
        if ready { rust_shutdown() }
    }

    private func physicalSize() -> (UInt32, UInt32) {
        let scale = CGFloat(density)
        return (UInt32(max(0, (view.bounds.width * scale).rounded())),
                UInt32(max(0, (view.bounds.height * scale).rounded())))
    }

    private func tick() {
        guard ready && !failed else { return }
        let size = physicalSize()
        rust_resize(size.0, size.1, density)
        let result = rust_render()
        if result == -2, let layer = view.layer as? CAMetalLayer {
            failed = !rust_replace_layer(Unmanaged.passUnretained(layer).toOpaque())
            if !failed { backingLayer = layer }
        } else if result < 0 {
            failed = true
            NSLog("Native Creator presentation failed")
        }
        serviceRequests()
        refreshTextState()
        #if os(macOS)
        let changed = rust_has_changes()
        if hasChanges != changed {
            hasChanges = changed
            view.window?.isDocumentEdited = changed
        }
        #endif
    }

    private func observeLifecycle() {
        let center = NotificationCenter.default
        #if os(macOS)
        for (name, focused) in [(NSWindow.didBecomeKeyNotification, true), (NSWindow.didResignKeyNotification, false)] {
            observers.append(center.addObserver(forName: name, object: nil, queue: .main) { [weak self] note in
                guard let self, let window = note.object as? NSWindow, window === self.view.window else { return }
                rust_focus(focused)
            })
        }
        for (name, active) in [(NSWindow.didMiniaturizeNotification, false), (NSWindow.didDeminiaturizeNotification, true)] {
            observers.append(center.addObserver(forName: name, object: nil, queue: .main) { [weak self] note in
                guard let self, let window = note.object as? NSWindow, window === self.view.window else { return }
                if active { rust_resume() } else { rust_suspend() }
            })
        }
        #else
        observers.append(center.addObserver(forName: UIApplication.didEnterBackgroundNotification, object: nil, queue: .main) { _ in rust_suspend() })
        observers.append(center.addObserver(forName: UIApplication.willEnterForegroundNotification, object: nil, queue: .main) { _ in rust_resume() })
        #endif
    }

    private func serviceRequests() {
        guard let source = rust_take_requests() else { return }
        defer { rust_free_string(source) }
        guard let data = String(cString: source).data(using: .utf8),
              let requests = try? JSONSerialization.jsonObject(with: data) as? [[String: Any]] else { return }
        for request in requests {
            let id = (request["request"] as? NSNumber)?.uint64Value
            switch request["kind"] as? String {
            case "title":
                #if os(macOS)
                view.window?.title = request["title"] as? String ?? "Eldiron Creator"
                #endif
            case "error": NSLog("Creator: %@", request["message"] as? String ?? "Unknown error")
            case "exit":
                #if os(macOS)
                NSApp.terminate(nil)
                #else
                rust_suspend()
                #endif
            case "read":
                guard let id else { continue }
                #if os(macOS)
                let text = NSPasteboard.general.string(forType: .string)
                #else
                let text = UIPasteboard.general.string
                #endif
                if let text { text.withCString { rust_clipboard_complete(id, false, true, $0) } }
                else { rust_clipboard_complete(id, false, false, nil) }
            case "write":
                guard let text = request["text"] as? String else { continue }
                #if os(macOS)
                NSPasteboard.general.clearContents()
                let success = NSPasteboard.general.setString(text, forType: .string)
                #else
                UIPasteboard.general.string = text
                let success = true
                #endif
                if let id { rust_clipboard_complete(id, true, success, nil) }
            default: break
            }
        }
    }

    func draw(in view: MTKView) { tick() }
    func mtkView(_ view: MTKView, drawableSizeWillChange size: CGSize) {
        guard ready else { return }
        let size = physicalSize()
        rust_resize(size.0, size.1, density)
    }
    func needsUpdate() { tick() }
    func updateOnce() { tick() }
}


extension Renderer: NativeTextHost {
    private func decode<T: Decodable>(_ type: T.Type, _ pointer: UnsafeMutablePointer<CChar>?) -> T? {
        guard let pointer else { return nil }
        defer { rust_free_string(pointer) }
        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase
        return try? decoder.decode(type, from: Data(String(cString: pointer).utf8))
    }
    private func refreshTextState() {
        struct TextState: Decodable { let textInput: NativeTextInput? }
        nativeInputState = decode(TextState.self, rust_host_state())?.textInput
        view.synchronizeNativeText(host: self)
    }
    func nativeTextSnapshot() -> NativeTextSnapshot? {
        guard let input = nativeInputState else { sourceSnapshot = nil; return nil }
        if var cached = sourceSnapshot, cached.target == input.target, cached.revision == input.revision {
            if let selection = input.selection { cached.selection = selection }
            sourceSnapshot = cached
            return cached
        }
        sourceSnapshot = decode(NativeTextSnapshot.self, rust_text_snapshot())
        return sourceSnapshot
    }
    private func endpoints(_ range: NSRange?) -> (Int, Int)? {
        guard let range, range.location != NSNotFound else { return (-1, -1) }
        guard range.location >= 0, range.length >= 0 else { return nil }
        let end = range.location.addingReportingOverflow(range.length)
        return end.overflow ? nil : (range.location, end.partialValue)
    }
    func queryNativeText(source: NativeTextSnapshot, range: NSRange?, point: CGPoint?) -> NativeTextAnswer? {
        guard let range = endpoints(range) else { return nil }
        return decode(NativeTextAnswer.self, rust_text_query(source.target, source.revision,
            point == nil ? 0 : 1, range.0, range.1, Float(point?.x ?? 0), Float(point?.y ?? 0)))
    }
    func navigateNativeText(source: NativeTextSnapshot, position: Int, direction: Int, count: Int) -> Int? {
        guard (0...3).contains(direction), count >= 0 else { return nil }
        return decode(NativeTextAnswer.self, rust_text_query(source.target, source.revision,
            UInt32(direction + 2), position, count, 0, 0))?.position
    }
    func editNativeText(kind: UInt32, source: NativeTextSnapshot, range: NSRange?, text: String, selection: NSRange?) -> Bool {
        guard var range = endpoints(range), let cursor = endpoints(selection) else { return false }
        if (kind == 5 || kind == 6) && range.0 == -1 && source.selection.count == 2 {
            range = (source.selection[0], source.selection[1])
        }
        let result = Array(text.utf8).withUnsafeBufferPointer {
            rust_text_edit(source.target, source.revision, kind, range.0, range.1,
                $0.baseAddress, UInt($0.count), cursor.0, cursor.1)
        }
        refreshTextState()
        return result > 0
    }
}
