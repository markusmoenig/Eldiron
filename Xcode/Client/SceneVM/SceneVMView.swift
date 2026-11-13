import SwiftUI
import QuartzCore

struct SceneVMView: View {
    @ObservedObject var document: SceneVMDocument
    var body: some View { PlatformView(document: document) }
}

#if os(macOS)
import AppKit
struct PlatformView: NSViewRepresentable {
    @ObservedObject var document: SceneVMDocument
    func makeNSView(context: Context) -> MetalContainer {
        let view = MetalContainer()
        view.document = document
        return view
    }
    func updateNSView(_ view: MetalContainer, context: Context) { view.document = document }
    static func dismantleNSView(_ view: MetalContainer, coordinator: ()) { view.close() }
}

final class MetalContainer: NSView {
    private let metalLayer = CAMetalLayer()
    private var handle: SceneVMHandle?
    private var timer: Timer?
    private var loadedSource: Data?
    private var observations: [NSObjectProtocol] = []
    private var hiddenCursor = false
    private var synchronizingNativeText = false
    private var nativeClient: NativeTextClient?
    private var nativeContext: NativeTextContext?
    override var inputContext: NSTextInputContext? { nativeContext }
    weak var document: SceneVMDocument? {
        didSet {
            if oldValue !== document {
                nativeClient?.cancelForDeactivation(); nativeContext?.discardMarkedText(); nativeContext?.deactivate()
                nativeClient = nil; nativeContext = nil
                handle?.focus(false); handle?.suspend()
                handle?.onState = nil; handle?.onError = nil
                handle = nil; loadedSource = nil
                setNeedsLayoutForDocument()
            }
            loadProjectIfNeeded()
        }
    }
    private func setNeedsLayoutForDocument() { needsLayout = true }
    override var acceptsFirstResponder: Bool { true }

    override init(frame: NSRect) {
        super.init(frame: frame)
        wantsLayer = true
        layer = metalLayer
        // Rust configures the layer's Metal device/format and exclusively owns
        // drawable acquisition and presentation. Swift supplies layout/events.
    }
    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }
    func close() {
        timer?.invalidate(); timer = nil
        observations.forEach(NotificationCenter.default.removeObserver); observations.removeAll()
        if hiddenCursor { NSCursor.unhide(); hiddenCursor = false }
        nativeClient?.cancelForDeactivation(); nativeContext?.discardMarkedText(); nativeContext?.deactivate()
        nativeClient = nil; nativeContext = nil
        handle?.focus(false)
        handle?.suspend()
        handle?.onState = nil; handle?.onError = nil
        handle = nil
        loadedSource = nil
    }
    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        observations.forEach(NotificationCenter.default.removeObserver); observations.removeAll()
        guard let window else { handle?.suspend(); timer?.invalidate(); timer = nil; return }
        for name in [NSWindow.didBecomeKeyNotification, NSWindow.didResignKeyNotification] {
            observations.append(NotificationCenter.default.addObserver(forName: name, object: window, queue: .main) { [weak self] _ in
                MainActor.assumeIsolated {
                    guard let self else { return }
                    self.handle?.focus(self.window?.isKeyWindow == true)
                    if self.window?.isKeyWindow != true && self.hiddenCursor { NSCursor.unhide(); self.hiddenCursor = false }
                }
            })
        }
        handle?.resume()
        if timer == nil {
            timer = Timer(timeInterval: 1.0 / 60.0, repeats: true) { [weak self] _ in
                MainActor.assumeIsolated { self?.handle?.render() }
            }
            RunLoop.main.add(timer!, forMode: .common)
        }
        needsLayout = true
    }
    override func viewDidChangeBackingProperties() { super.viewDidChangeBackingProperties(); needsLayout = true }
    override func layout() {
        super.layout()
        let scale = window?.backingScaleFactor ?? 1
        metalLayer.frame = bounds
        metalLayer.contentsScale = scale
        metalLayer.drawableSize = CGSize(width: bounds.width * scale, height: bounds.height * scale)
        if handle == nil && bounds.width > 0 && bounds.height > 0 {
            if let retained = document?.sceneVMHandle {
                guard retained.replaceLayer(metalLayer, size: bounds.size, scale: scale) else { return }
                handle = retained
                loadedSource = document?.projectData
            } else { handle = SceneVMHandle(layer: metalLayer, size: bounds.size, scale: scale) }
            guard let handle else { document?.errorMessage = "Unable to create the player surface"; return }
            handle.resume()
            handle.onError = { [weak self] message in self?.document?.errorMessage = message }
            handle.onState = { [weak self] state in self?.hostState(state) }
            document?.sceneVMHandle = handle
            handle.synchronizeHost()
            handle.focus(window?.isKeyWindow == true)
            loadProjectIfNeeded()
        } else { handle?.resize(to: bounds.size, scale: scale) }
    }
    private func loadProjectIfNeeded() {
        guard let handle, let document, !document.projectData.isEmpty, loadedSource != document.projectData else { return }
        loadedSource = document.projectData
        _ = handle.loadProject(data: document.projectData)
    }
    private func hostState(_ state: NativeHostState) {
        guard !synchronizingNativeText else { return }
        synchronizingNativeText = true
        defer { synchronizingNativeText = false }
        window?.title = state.title
        let hide = !state.cursorVisible && window?.isKeyWindow == true
        if hide != hiddenCursor { if hide { NSCursor.hide() } else { NSCursor.unhide() }; hiddenCursor = hide }
        switch state.cursor {
        case "Text", "VerticalText": NSCursor.iBeam.set()
        case "Hand": NSCursor.pointingHand.set()
        case "Crosshair", "Cell": NSCursor.crosshair.set()
        case "Grab": NSCursor.openHand.set()
        case "Grabbing": NSCursor.closedHand.set()
        case "NotAllowed", "NoDrop": NSCursor.operationNotAllowed.set()
        case "EWResize", "EResize", "WResize", "ColResize": NSCursor.resizeLeftRight.set()
        case "NSResize", "NResize", "SResize", "RowResize": NSCursor.resizeUpDown.set()
        default: NSCursor.arrow.set()
        }
        if nativeClient?.target != state.textInput?.target {
            nativeClient?.cancelForDeactivation(); nativeContext?.discardMarkedText(); nativeContext?.deactivate()
            nativeClient = nil; nativeContext = nil
            if let target = state.textInput?.target, let handle {
                let client = NativeTextClient(target: target, host: handle, view: self) { [weak self] selector in self?.doCommand(by: selector) }
                nativeClient = client
                nativeContext = NativeTextContext(client: client)
                if window?.firstResponder === self { nativeContext?.activate() }
            }
        }
        nativeContext?.invalidateCharacterCoordinates()
    }
    private func position(_ event: NSEvent) -> CGPoint {
        let p = convert(event.locationInWindow, from: nil)
        return CGPoint(x: p.x, y: bounds.height - p.y)
    }
    private func modifiers(_ flags: NSEvent.ModifierFlags) {
        handle?.modifiers((flags.contains(.shift) ? 1 : 0) | (flags.contains(.control) ? 2 : 0) | (flags.contains(.option) ? 4 : 0) | (flags.contains(.command) ? 8 : 0))
    }
    override func flagsChanged(with event: NSEvent) { modifiers(event.modifierFlags) }
    override func updateTrackingAreas() {
        super.updateTrackingAreas()
        trackingAreas.forEach(removeTrackingArea)
        addTrackingArea(NSTrackingArea(rect: .zero, options: [.mouseMoved, .mouseEnteredAndExited, .activeInKeyWindow, .inVisibleRect], owner: self))
    }
    override func mouseMoved(with event: NSEvent) {
        modifiers(event.modifierFlags)
        let p = position(event)
        handle?.mouseMove(x: p.x, y: p.y)
        handle?.mouseMotion(dx: event.deltaX, dy: event.deltaY)
    }
    override func mouseExited(with event: NSEvent) { handle?.mouseLeave() }
    override func mouseDown(with event: NSEvent) { pointer(event, true) }
    override func mouseUp(with event: NSEvent) { pointer(event, false) }
    override func rightMouseDown(with event: NSEvent) { pointer(event, true) }
    override func rightMouseUp(with event: NSEvent) { pointer(event, false) }
    override func otherMouseDown(with event: NSEvent) { pointer(event, true) }
    override func otherMouseUp(with event: NSEvent) { pointer(event, false) }
    private func pointer(_ event: NSEvent, _ pressed: Bool) {
        if pressed { window?.makeFirstResponder(self) }
        modifiers(event.modifierFlags)
        let p = position(event)
        let button = UInt16(clamping: event.buttonNumber)
        if pressed { handle?.mouseDown(x: p.x, y: p.y, button: button) } else { handle?.mouseUp(x: p.x, y: p.y, button: button) }
    }
    override func mouseDragged(with event: NSEvent) { mouseMoved(with: event) }
    override func rightMouseDragged(with event: NSEvent) { mouseMoved(with: event) }
    override func otherMouseDragged(with event: NSEvent) { mouseMoved(with: event) }
    override func scrollWheel(with event: NSEvent) { modifiers(event.modifierFlags); handle?.scroll(dx: event.scrollingDeltaX, dy: event.scrollingDeltaY) }
    override func magnify(with event: NSEvent) { handle?.pinch(scale: 1 + event.magnification, center: position(event)) }
    private func physical(_ code: UInt16) -> UInt32 {
        // macOS virtual key positions, independent of keyboard layout/text.
        let letters: [UInt16: UInt32] = [0:65,1:83,2:68,3:70,4:72,5:71,6:90,7:88,8:67,9:86,11:66,12:81,13:87,14:69,15:82,16:89,17:84,31:79,32:85,34:73,35:80,37:76,38:74,40:75,45:78,46:77]
        if let value = letters[code] { return value }
        let others: [UInt16:UInt32] = [18:49,19:50,20:51,21:52,23:53,22:54,26:55,28:56,25:57,29:48,36:0x100,53:0x101,51:0x102,48:0x103,49:0x104,117:0x105,123:0x106,124:0x107,126:0x108,125:0x109,115:0x10a,119:0x10b,116:0x10c,121:0x10d,122:0x201,120:0x202,99:0x203,118:0x204,96:0x205,97:0x206,98:0x207,100:0x208,101:0x209,109:0x20a,103:0x20b,111:0x20c]
        return others[code] ?? (0x10000 + UInt32(code))
    }
    override func keyDown(with event: NSEvent) {
        modifiers(event.modifierFlags)
        let client = nativeClient
        let context = nativeContext
        client?.beginKey()
        defer { client?.endKey() }
        if client?.hasMarkedText() == true { _ = context?.handleEvent(event); return }
        let consumed = handle?.key(physical(event.keyCode), pressed: true, repeated: event.isARepeat) == true
        if !consumed && client === nativeClient { _ = context?.handleEvent(event) }
    }
    override func keyUp(with event: NSEvent) { modifiers(event.modifierFlags); _ = handle?.key(physical(event.keyCode), pressed: false) }
    private func shortcut(_ code: UInt32, shift: Bool = false) {
        handle?.modifiers(8 | (shift ? 1 : 0))
        _ = handle?.key(code, pressed: true); _ = handle?.key(code, pressed: false)
        handle?.modifiers(0)
    }
    @objc func copy(_ sender: Any?) { shortcut(67) }
    @objc func cut(_ sender: Any?) { shortcut(88) }
    @objc func paste(_ sender: Any?) { shortcut(86) }
    @objc override func selectAll(_ sender: Any?) { shortcut(65) }
    @objc func undo(_ sender: Any?) { shortcut(90) }
    @objc func redo(_ sender: Any?) { shortcut(90, shift: true) }

    override func doCommand(by selector: Selector) {
        let codes: [String:UInt32] = ["insertNewline:":0x100,"cancelOperation:":0x101,"deleteBackward:":0x102,"insertTab:":0x103,"deleteForward:":0x105,"moveLeft:":0x106,"moveRight:":0x107,"moveUp:":0x108,"moveDown:":0x109]
        if let code = codes[NSStringFromSelector(selector)] { _ = handle?.key(code, pressed: true); _ = handle?.key(code, pressed: false) }
    }
}
#else
import UIKit
struct PlatformView: UIViewRepresentable {
    @ObservedObject var document: SceneVMDocument
    func makeUIView(context: Context) -> MetalContainer { let view = MetalContainer(); view.document = document; return view }
    func updateUIView(_ view: MetalContainer, context: Context) { view.document = document }
    static func dismantleUIView(_ view: MetalContainer, coordinator: ()) { view.close() }
}
final class MetalContainer: UIView {
    override class var layerClass: AnyClass { CAMetalLayer.self }
    private var metalLayer: CAMetalLayer { layer as! CAMetalLayer }
    private var handle: SceneVMHandle?
    private var timer: Timer?
    private var observations: [NSObjectProtocol] = []
    private var loadedSource: Data?
    private var nativeClient: NativeUIKitTextClient?
    weak var document: SceneVMDocument? {
        didSet {
            if oldValue !== document {
                nativeClient?.deactivate(); nativeClient?.removeFromSuperview(); nativeClient = nil
                handle?.focus(false); handle?.suspend()
                handle?.onState = nil; handle?.onError = nil
                handle = nil; loadedSource = nil
                setNeedsLayoutForDocument()
            }
            loadProjectIfNeeded()
        }
    }
    private func setNeedsLayoutForDocument() { setNeedsLayout() }
    override var canBecomeFirstResponder: Bool { true }
    override init(frame: CGRect) {
        super.init(frame: frame)
        addGestureRecognizer(UIPinchGestureRecognizer(target: self, action: #selector(pinch(_:))))
        for name in [UIApplication.willResignActiveNotification, UIApplication.didBecomeActiveNotification] {
            observations.append(NotificationCenter.default.addObserver(forName: name, object: nil, queue: .main) { [weak self] notification in
                MainActor.assumeIsolated {
                    if notification.name == UIApplication.willResignActiveNotification { self?.handle?.focus(false); self?.handle?.suspend() }
                    else { self?.handle?.resume(); self?.handle?.focus(true) }
                }
            })
        }
    }
    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }
    func close() {
        timer?.invalidate(); timer = nil
        observations.forEach(NotificationCenter.default.removeObserver); observations.removeAll()
        nativeClient?.deactivate(); nativeClient?.removeFromSuperview(); nativeClient = nil
        handle?.focus(false); handle?.suspend()
        handle?.onState = nil; handle?.onError = nil
        handle = nil; loadedSource = nil
    }
    override func didMoveToWindow() {
        super.didMoveToWindow()
        guard window != nil else { handle?.suspend(); timer?.invalidate(); timer = nil; return }
        handle?.resume()
        if timer == nil {
            timer = Timer(timeInterval: 1.0 / 60.0, repeats: true) { [weak self] _ in MainActor.assumeIsolated { self?.handle?.render() } }
            RunLoop.main.add(timer!, forMode: .common)
        }
    }
    override func layoutSubviews() {
        super.layoutSubviews()
        let scale = window?.screen.scale ?? 1
        metalLayer.frame = bounds; metalLayer.contentsScale = scale
        metalLayer.drawableSize = CGSize(width: bounds.width * scale, height: bounds.height * scale)
        if handle == nil && bounds.width > 0 && bounds.height > 0 {
            if let retained = document?.sceneVMHandle {
                guard retained.replaceLayer(metalLayer, size: bounds.size, scale: scale) else { return }
                handle = retained
                loadedSource = document?.projectData
            } else { handle = SceneVMHandle(layer: metalLayer, size: bounds.size, scale: scale) }
            guard let handle else { document?.errorMessage = "Unable to create the player surface"; return }
            handle.resume()
            handle.onError = { [weak self] error in self?.document?.errorMessage = error }
            handle.onState = { [weak self] state in
                guard let self else { return }
                self.hostState(state)
            }
            document?.sceneVMHandle = handle
            handle.synchronizeHost()
            handle.focus(window != nil)
            loadProjectIfNeeded()
        } else { handle?.resize(to: bounds.size, scale: scale) }
    }
    private func loadProjectIfNeeded() {
        guard let handle, let document, !document.projectData.isEmpty, loadedSource != document.projectData else { return }
        loadedSource = document.projectData
        _ = handle.loadProject(data: document.projectData)
    }
    private func hostState(_ state: NativeHostState) {
        if nativeClient?.target != state.textInput?.target {
            let old = nativeClient; nativeClient = nil
            old?.deactivate(); old?.removeFromSuperview()
            if let target = state.textInput?.target, let handle {
                let client = NativeUIKitTextClient(target: target, host: handle, view: self)
                addSubview(client); nativeClient = client
                if window != nil && UIApplication.shared.applicationState == .active { client.becomeFirstResponder() }
            } else if window != nil && UIApplication.shared.applicationState == .active { becomeFirstResponder() }
        }
        nativeClient?.synchronize()
    }
    private func physical(_ code: Int) -> UInt32 {
        switch code {
        case 4...29: return UInt32(65 + code - 4)
        case 30...38: return UInt32(49 + code - 30)
        case 39: return 48
        case 40: return 0x100
        case 41: return 0x101
        case 42: return 0x102
        case 43: return 0x103
        case 44: return 0x104
        case 76: return 0x105
        case 80: return 0x106
        case 79: return 0x107
        case 82: return 0x108
        case 81: return 0x109
        case 74: return 0x10a
        case 77: return 0x10b
        case 75: return 0x10c
        case 78: return 0x10d
        case 58...69: return UInt32(0x201 + code - 58)
        default: return UInt32(0x10000 + code)
        }
    }
    private func presses(_ presses: Set<UIPress>, pressed: Bool) -> Bool {
        // Marked text is owned by UIKit's input method. Its physical navigation
        // and delete keys must reach UITextInput instead of being swallowed by
        // the shared controller's deliberate "IME owns keys" response.
        if nativeClient?.markedTextRange != nil { return false }
        var consumed = false
        for press in presses {
            guard let key = press.key else { continue }
            let f = key.modifierFlags
            handle?.modifiers((f.contains(.shift) ? 1 : 0) | (f.contains(.control) ? 2 : 0) | (f.contains(.alternate) ? 4 : 0) | (f.contains(.command) ? 8 : 0))
            consumed = handle?.key(physical(key.keyCode.rawValue), pressed: pressed) == true || consumed
        }
        return consumed
    }
    override func pressesBegan(_ presses: Set<UIPress>, with event: UIPressesEvent?) { if !self.presses(presses, pressed: true) { super.pressesBegan(presses, with: event) } }
    override func pressesEnded(_ presses: Set<UIPress>, with event: UIPressesEvent?) { _ = self.presses(presses, pressed: false); super.pressesEnded(presses, with: event) }
    override func touchesBegan(_ touches: Set<UITouch>, with event: UIEvent?) {
        guard let p = touches.first?.location(in: self) else { return }
        if nativeClient == nil { becomeFirstResponder() }
        handle?.mouseDown(x: p.x, y: p.y)
        // Reacquire the same logical editor after a keyboard dismissal or view
        // reattachment, even when its Rust owner did not change on this touch.
        nativeClient?.becomeFirstResponder()
    }
    override func touchesMoved(_ touches: Set<UITouch>, with event: UIEvent?) { guard let p = touches.first?.location(in: self) else { return }; handle?.mouseMove(x: p.x, y: p.y) }
    override func touchesEnded(_ touches: Set<UITouch>, with event: UIEvent?) { guard let p = touches.first?.location(in: self) else { return }; handle?.mouseUp(x: p.x, y: p.y) }
    override func touchesCancelled(_ touches: Set<UITouch>, with event: UIEvent?) { handle?.focus(false); handle?.focus(true) }
    @objc private func pinch(_ gesture: UIPinchGestureRecognizer) { handle?.pinch(scale: gesture.scale, center: gesture.location(in: self)); gesture.scale = 1 }
}
#endif
