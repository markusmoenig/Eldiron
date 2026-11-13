//
//  RMKTView.swift
//  Xcode2Rust iOS
//
//  Created by Markus Moenig on 30/10/22.
//

import MetalKit

public class RMTKView       : MTKView
{
    var renderer            : Renderer!

    private var isSynchronizingNativeText = false

    var keysDown            : [Float] = []

    var mouseIsDown         : Bool = false
    var mousePos            = SIMD2<Float>(0, 0)

    var hasTap              : Bool = false
    var hasDoubleTap        : Bool = false

    var buttonDown          : String? = nil
    var swipeDirection      : String? = nil

    func reset()
    {
        keysDown = []
        mouseIsDown = false
        hasTap  = false
        hasDoubleTap  = false
        buttonDown = nil
        swipeDirection = nil
    }

    #if os(OSX)

    // --- Key States
    var shiftIsDown     : Bool = false
    var commandIsDown   : Bool = false
    var controlIsDown   : Bool = false
    var altIsDown       : Bool = false

    override public var acceptsFirstResponder: Bool { return true }

    /// To get continuous mouse events on macOS
    override public func updateTrackingAreas()
    {
        super.updateTrackingAreas()
        trackingAreas.forEach { removeTrackingArea($0) }
        let options : NSTrackingArea.Options = [.mouseEnteredAndExited, .mouseMoved, .activeInKeyWindow]
        let trackingArea = NSTrackingArea(rect: self.bounds, options: options,
                                      owner: self, userInfo: nil)
        self.addTrackingArea(trackingArea)
    }

    func platformInit()
    {
    }

    func setRightMouseCapture(_ active: Bool)
    {
        _ = CGAssociateMouseAndMouseCursorPosition(boolean_t(active ? 0 : 1))
    }

    func setMousePos(_ event: NSEvent)
    {
        let location = convert(event.locationInWindow, from: nil)
        mousePos.x = Float(location.x)
        mousePos.y = Float(isFlipped ? location.y : bounds.height - location.y)
    }

    private var nativeClient: NativeTextClient?
    private var nativeContext: NativeTextContext?
    private var acceptsNativeTextEvents = true
    override public var inputContext: NSTextInputContext? { nativeContext }
    override public func becomeFirstResponder() -> Bool {
        let accepted = super.becomeFirstResponder()
        acceptsNativeTextEvents = accepted
        if accepted { nativeContext?.activate() }
        return accepted
    }
    override public func resignFirstResponder() -> Bool {
        let accepted = super.resignFirstResponder()
        if accepted {
            acceptsNativeTextEvents = false
            nativeClient?.cancelForDeactivation()
            nativeContext?.discardMarkedText()
            nativeContext?.deactivate()
            nativeClient = nil; nativeContext = nil
        }
        return accepted
    }
    func synchronizeNativeText(host: any NativeTextHost) {
        guard !isSynchronizingNativeText else { return }
        isSynchronizingNativeText = true
        defer { isSynchronizingNativeText = false }
        let target = acceptsNativeTextEvents && window?.isKeyWindow == true ? host.nativeInputState?.target : nil
        if nativeClient?.target != target {
            nativeClient?.cancelForDeactivation()
            nativeContext?.discardMarkedText()
            nativeContext?.deactivate()
            nativeClient = nil; nativeContext = nil
            if let target {
                let client = NativeTextClient(target: target, host: host, view: self) { [weak self] in self?.doCommand(by: $0) }
                nativeClient = client
                nativeContext = NativeTextContext(client: client)
                if window?.firstResponder === self { nativeContext?.activate() }
            }
        }
        nativeContext?.invalidateCharacterCoordinates()
    }
    private func physical(_ code: UInt16) -> UInt32 {
        // macOS virtual key positions, independent of keyboard layout/text.
        let letters: [UInt16: UInt32] = [0:65,1:83,2:68,3:70,4:72,5:71,6:90,7:88,8:67,9:86,11:66,12:81,13:87,14:69,15:82,16:89,17:84,31:79,32:85,34:73,35:80,37:76,38:74,40:75,45:78,46:77]
        if let value = letters[code] { return value }
        let others: [UInt16:UInt32] = [18:49,19:50,20:51,21:52,23:53,22:54,26:55,28:56,25:57,29:48,36:0x100,53:0x101,51:0x102,48:0x103,49:0x104,117:0x105,123:0x106,124:0x107,126:0x108,125:0x109,115:0x10a,119:0x10b,116:0x10c,121:0x10d,122:0x201,120:0x202,99:0x203,118:0x204,96:0x205,97:0x206,98:0x207,100:0x208,101:0x209,109:0x20a,103:0x20b,111:0x20c]
        return others[code] ?? (0x10000 + UInt32(code))
    }

    override public func keyDown(with event: NSEvent) {
        _ = rust_key_modifier_changed(event.modifierFlags.contains(.shift), event.modifierFlags.contains(.control), event.modifierFlags.contains(.option), event.modifierFlags.contains(.command))
        let client = nativeClient, context = nativeContext
        client?.beginKey()
        defer { client?.endKey() }
        if client?.hasMarkedText() == true { _ = context?.handleEvent(event); renderer.needsUpdate(); return }
        let consumed = (event.charactersIgnoringModifiers ?? "").withCString { rust_physical_key(physical(event.keyCode), true, event.isARepeat, $0) }
        if !consumed && client === nativeClient { _ = context?.handleEvent(event) }
        renderer.needsUpdate()
    }
    override public func keyUp(with event: NSEvent) {
        _ = (event.charactersIgnoringModifiers ?? "").withCString { rust_physical_key(physical(event.keyCode), false, false, $0) }
        renderer.needsUpdate()
    }
    override public func doCommand(by selector: Selector) {
        let codes: [String: UInt32] = ["insertNewline:":0x100,"cancelOperation:":0x101,"deleteBackward:":0x102,"insertTab:":0x103,"deleteForward:":0x105,"moveLeft:":0x106,"moveRight:":0x107,"moveUp:":0x108,"moveDown:":0x109]
        if let code = codes[NSStringFromSelector(selector)] {
            _ = rust_physical_key(code, true, false, nil)
            _ = rust_physical_key(code, false, false, nil)
        }
        renderer.needsUpdate()
    }

    override public func mouseDown(with event: NSEvent) {
        window?.makeFirstResponder(self)
        setMousePos(event)
        _ = rust_touch_down(mousePos.x, mousePos.y)
        renderer.needsUpdate()
    }

    override public func mouseDragged(with event: NSEvent) {
        setMousePos(event)
        _ = rust_touch_dragged(mousePos.x, mousePos.y)
        _ = rust_mouse_motion(Float(event.deltaX), Float(event.deltaY))
        renderer.needsUpdate()
    }

    override public func mouseMoved(with event: NSEvent) {
        setMousePos(event)
        _ = rust_hover(mousePos.x, mousePos.y)
        _ = rust_mouse_motion(Float(event.deltaX), Float(event.deltaY))
        renderer.needsUpdate()
    }

    override public func mouseExited(with event: NSEvent) {
        _ = rust_mouse_leave()
        renderer.needsUpdate()
    }

    override public func mouseUp(with event: NSEvent) {
        setMousePos(event)
        _ = rust_touch_up(mousePos.x, mousePos.y)
        renderer.needsUpdate()
    }

    override public func rightMouseDown(with event: NSEvent) {
        window?.makeFirstResponder(self)
        setMousePos(event)
        _ = rust_right_touch_down(mousePos.x, mousePos.y)
        setRightMouseCapture(true)
        renderer.needsUpdate()
    }

    override public func rightMouseDragged(with event: NSEvent) {
        if rust_mouse_motion(Float(event.deltaX), Float(event.deltaY)) {
            renderer.needsUpdate()
            return
        }

        setMousePos(event)
        _ = rust_right_touch_dragged(mousePos.x, mousePos.y)
        renderer.needsUpdate()
    }

    override public func rightMouseUp(with event: NSEvent) {
        setRightMouseCapture(false)
        setMousePos(event)
        _ = rust_right_touch_up(mousePos.x, mousePos.y)
        renderer.needsUpdate()
    }

    override public func flagsChanged(with event: NSEvent) {
        //https://stackoverflow.com/questions/9268045/how-can-i-detect-that-the-shift-key-has-been-pressed
        if event.modifierFlags.contains(.shift) {
            shiftIsDown = true
        } else {
            shiftIsDown = false
        }
        if event.modifierFlags.contains(.option) {
            altIsDown = true
        } else {
            altIsDown = false
        }
        if event.modifierFlags.contains(.control) {
            controlIsDown = true
        } else {
            controlIsDown = false
        }
        if event.modifierFlags.contains(.command) {
            commandIsDown = true
        } else {
            commandIsDown = false
        }

        if rust_key_modifier_changed(shiftIsDown, controlIsDown, altIsDown, commandIsDown) {
            renderer.needsUpdate()
        }
    }

    override public func scrollWheel(with event: NSEvent) {
        if event.hasPreciseScrollingDeltas {
            _ = rust_touch_precise_scroll(Float(event.scrollingDeltaX), Float(event.scrollingDeltaY))
        } else {
            _ = rust_touch_wheel(Float(event.scrollingDeltaX), Float(event.scrollingDeltaY))
        }
        renderer.needsUpdate()
    }

    override public func magnify(with event: NSEvent) {
        _ = rust_pinch(Float(event.magnification))
        renderer.needsUpdate()
    }
    #elseif os(iOS)

    override public var canBecomeFirstResponder: Bool { true }
    private var nativeClient: NativeUIKitTextClient?
    func synchronizeNativeText(host: any NativeTextHost) {
        guard !isSynchronizingNativeText else { return }
        isSynchronizingNativeText = true
        defer { isSynchronizingNativeText = false }
        if nativeClient?.target != host.nativeInputState?.target {
            let old = nativeClient; nativeClient = nil
            old?.deactivate(); old?.removeFromSuperview()
            if let target = host.nativeInputState?.target {
                let client = NativeUIKitTextClient(target: target, host: host, view: self)
                nativeClient = client; addSubview(client)
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
    private func nativePresses(_ presses: Set<UIPress>, pressed: Bool) -> Bool {
        if nativeClient?.markedTextRange != nil { return false }
        var consumed = false
        for press in presses {
            guard let key = press.key else { continue }
            let flags = key.modifierFlags
            _ = rust_key_modifier_changed(flags.contains(.shift), flags.contains(.control), flags.contains(.alternate), flags.contains(.command))
            consumed = key.charactersIgnoringModifiers.withCString { rust_physical_key(physical(key.keyCode.rawValue), pressed, false, $0) } || consumed
        }
        renderer.needsUpdate()
        return consumed
    }
    override public func pressesBegan(_ presses: Set<UIPress>, with event: UIPressesEvent?) {
        if !nativePresses(presses, pressed: true) { super.pressesBegan(presses, with: event) }
    }
    override public func pressesEnded(_ presses: Set<UIPress>, with event: UIPressesEvent?) {
        if !nativePresses(presses, pressed: false) { super.pressesEnded(presses, with: event) }
    }
    override public func pressesCancelled(_ presses: Set<UIPress>, with event: UIPressesEvent?) {
        _ = nativePresses(presses, pressed: false)
        super.pressesCancelled(presses, with: event)
    }

    func platformInit()
    {
        let tapRecognizer = UITapGestureRecognizer(target: self, action:(#selector(self.handleTapGesture(_:))))
        tapRecognizer.numberOfTapsRequired = 1
        addGestureRecognizer(tapRecognizer)

        let pinchRecognizer = UIPinchGestureRecognizer(target: self, action:(#selector(self.handlePinchGesture(_:))))
        addGestureRecognizer(pinchRecognizer)
    }

    var lastPinch : Float = 1

    @objc func handlePinchGesture(_ recognizer: UIPinchGestureRecognizer) {
        _ = rust_pinch(Float(recognizer.scale - 1))
        recognizer.scale = 1
        renderer.needsUpdate()
    }

    @objc func handleTapGesture(_ recognizer: UITapGestureRecognizer)
    {
        if recognizer.numberOfTouches == 1 {
            hasTap = true
            DispatchQueue.main.asyncAfter(deadline: .now() + 1.0 / 60.0) {
                self.hasTap = false
            }
        } else
        if recognizer.numberOfTouches >= 1 {
            hasDoubleTap = true
            DispatchQueue.main.asyncAfter(deadline: .now() + 1.0 / 60.0) {
                self.hasDoubleTap = false
            }
        }
    }

    func setMousePos(_ x: Float, _ y: Float)
    {
        mousePos.x = x
        mousePos.y = y

        //mousePos.x /= Float(bounds.width) / game.texture!.width// / game.scaleFactor
        //mousePos.y /= Float(bounds.height) / game.texture!.height// / game.scaleFactor
    }

    var firstTouch = SIMD2<Float>(0, 0)
    override public func touchesBegan(_ touches: Set<UITouch>, with event: UIEvent?) {
        mouseIsDown = true
        if let touch = touches.first {
            let point = touch.location(in: self)
            setMousePos(Float(point.x), Float(point.y))
            _ = rust_touch_down(mousePos.x, mousePos.y)
            renderer.needsUpdate()
        }
    }

    override public func touchesMoved(_ touches: Set<UITouch>, with event: UIEvent?) {
        if let touch = touches.first {
            let point = touch.location(in: self)
            setMousePos(Float(point.x), Float(point.y))
            _ = rust_touch_dragged(mousePos.x, mousePos.y)
            renderer.needsUpdate()
        }
    }

    override public func touchesEnded(_ touches: Set<UITouch>, with event: UIEvent?) {
        mouseIsDown = false
        if let touch = touches.first {
            let point = touch.location(in: self)
            setMousePos(Float(point.x), Float(point.y))
            _ = rust_touch_up(mousePos.x, mousePos.y)
            renderer.needsUpdate()
        }
    }

    #elseif os(tvOS)

    func platformInit()
    {
        var swipeRecognizer = UISwipeGestureRecognizer(target: self, action: #selector(swipedRight))
        swipeRecognizer.direction = .right
        addGestureRecognizer(swipeRecognizer)

        swipeRecognizer = UISwipeGestureRecognizer(target: self, action: #selector(swipedLeft))
        swipeRecognizer.direction = .left
        addGestureRecognizer(swipeRecognizer)

        swipeRecognizer = UISwipeGestureRecognizer(target: self, action: #selector(swipedUp))
        swipeRecognizer.direction = .up
        addGestureRecognizer(swipeRecognizer)

        swipeRecognizer = UISwipeGestureRecognizer(target: self, action: #selector(swipedDown))
        swipeRecognizer.direction = .down
        addGestureRecognizer(swipeRecognizer)
    }

    public override func pressesBegan(_ presses: Set<UIPress>, with event: UIPressesEvent?)
    {
        guard let buttonPress = presses.first?.type else { return }

        switch(buttonPress) {
            case .menu:
                buttonDown = "Menu"
            case .playPause:
                buttonDown = "Play/Pause"
            case .select:
                buttonDown = "Select"
            case .upArrow:
                buttonDown = "ArrowUp"
            case .downArrow:
                buttonDown = "ArrowDown"
            case .leftArrow:
                buttonDown = "ArrowLeft"
            case .rightArrow:
                buttonDown = "ArrowRight"
            default:
                print("Unkown Button", buttonPress)
        }
    }

    public override func pressesEnded(_ presses: Set<UIPress>, with event: UIPressesEvent?)
    {
        buttonDown = nil
    }

    @objc func swipedUp() {
       swipeDirection = "up"
    }

    @objc func swipedDown() {
       swipeDirection = "down"
    }

    @objc func swipedRight() {
       swipeDirection = "right"
    }

    @objc func swipedLeft() {
       swipeDirection = "left"
    }


    #endif
}
