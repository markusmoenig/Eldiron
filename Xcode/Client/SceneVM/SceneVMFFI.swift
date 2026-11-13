import Foundation
import QuartzCore

// C FFI imported directly via @_silgen_name to avoid a bridging header.
@_silgen_name("unified_app_runner_create")
func unified_app_runner_create(_ layer_ptr: UnsafeMutableRawPointer?, _ width: UInt32, _ height: UInt32, _ scale: Float) -> UnsafeMutableRawPointer?

@_silgen_name("unified_app_runner_destroy")
func unified_app_runner_destroy(_ vm: UnsafeMutableRawPointer?)

@_silgen_name("unified_app_runner_resize")
func unified_app_runner_resize(_ vm: UnsafeMutableRawPointer?, _ width: UInt32, _ height: UInt32, _ scale: Float)

@_silgen_name("unified_app_runner_render")
func unified_app_runner_render(_ vm: UnsafeMutableRawPointer?) -> Int32

@_silgen_name("unified_app_runner_mouse_down")
func unified_app_runner_mouse_down(_ vm: UnsafeMutableRawPointer?, _ x: Float, _ y: Float)

@_silgen_name("unified_app_runner_mouse_up")
func unified_app_runner_mouse_up(_ vm: UnsafeMutableRawPointer?, _ x: Float, _ y: Float)

@_silgen_name("unified_app_runner_mouse_move")
func unified_app_runner_mouse_move(_ vm: UnsafeMutableRawPointer?, _ x: Float, _ y: Float)

@_silgen_name("unified_app_runner_mouse_motion")
func unified_app_runner_mouse_motion(_ vm: UnsafeMutableRawPointer?, _ dx: Float, _ dy: Float)

@_silgen_name("unified_app_runner_mouse_leave")
func unified_app_runner_mouse_leave(_ vm: UnsafeMutableRawPointer?)

@_silgen_name("unified_app_runner_scroll")
func unified_app_runner_scroll(_ vm: UnsafeMutableRawPointer?, _ dx: Float, _ dy: Float)

@_silgen_name("unified_app_runner_pinch")
func unified_app_runner_pinch(_ vm: UnsafeMutableRawPointer?, _ scale: Float, _ center_x: Float, _ center_y: Float)

// Project Management
@_silgen_name("unified_app_runner_save_project")
func unified_app_runner_save_project(_ vm: UnsafeMutableRawPointer?, _ out_json: UnsafeMutablePointer<UnsafePointer<UInt8>?>?, _ out_len: UnsafeMutablePointer<Int>?) -> Int32

@_silgen_name("unified_app_runner_load_project")
func unified_app_runner_load_project(_ vm: UnsafeMutableRawPointer?, _ json_data: UnsafePointer<UInt8>?, _ json_len: Int) -> Int32

@_silgen_name("unified_app_runner_free_json")
func unified_app_runner_free_json(_ json_ptr: UnsafePointer<UInt8>?, _ json_len: Int)

@_silgen_name("unified_app_runner_has_unsaved_changes")
func unified_app_runner_has_unsaved_changes(_ vm: UnsafeMutableRawPointer?) -> Int32

@_silgen_name("unified_app_runner_export_data")
func unified_app_runner_export_data(_ vm: UnsafeMutableRawPointer?, _ format: UnsafePointer<UInt8>?, _ format_len: Int, _ out_data: UnsafeMutablePointer<UnsafePointer<UInt8>?>?, _ out_len: UnsafeMutablePointer<Int>?) -> Int32

@_silgen_name("unified_app_runner_import_data")
func unified_app_runner_import_data(_ vm: UnsafeMutableRawPointer?, _ data: UnsafePointer<UInt8>?, _ data_len: Int, _ file_type: UnsafePointer<UInt8>?, _ file_type_len: Int) -> Int32

@_silgen_name("unified_app_runner_free_data")
func unified_app_runner_free_data(_ data_ptr: UnsafePointer<UInt8>?, _ data_len: Int)

@_silgen_name("unified_app_runner_set_theme")
func unified_app_runner_set_theme(_ vm: UnsafeMutableRawPointer?, _ is_dark: Int32, _ width: UInt32, _ height: UInt32)

// Undo/Redo Support
@_silgen_name("unified_app_runner_undo")
func unified_app_runner_undo(_ vm: UnsafeMutableRawPointer?) -> Int32

@_silgen_name("unified_app_runner_redo")
func unified_app_runner_redo(_ vm: UnsafeMutableRawPointer?) -> Int32

@_silgen_name("unified_app_runner_can_undo")
func unified_app_runner_can_undo(_ vm: UnsafeMutableRawPointer?) -> Int32

@_silgen_name("unified_app_runner_can_redo")
func unified_app_runner_can_redo(_ vm: UnsafeMutableRawPointer?) -> Int32

@_silgen_name("unified_app_runner_undo_description")
func unified_app_runner_undo_description(_ vm: UnsafeMutableRawPointer?, _ out_str: UnsafeMutablePointer<UnsafePointer<UInt8>?>?, _ out_len: UnsafeMutablePointer<Int>?) -> Int32

@_silgen_name("unified_app_runner_redo_description")
func unified_app_runner_redo_description(_ vm: UnsafeMutableRawPointer?, _ out_str: UnsafeMutablePointer<UnsafePointer<UInt8>?>?, _ out_len: UnsafeMutablePointer<Int>?) -> Int32

@_silgen_name("unified_app_runner_free_string")
func unified_app_runner_free_string(_ str_ptr: UnsafePointer<UInt8>?, _ str_len: Int)

@_silgen_name("unified_app_runner_host_state")
func unified_app_runner_host_state(_ vm: UnsafeMutableRawPointer?, _ out: UnsafeMutablePointer<UnsafePointer<UInt8>?>?, _ len: UnsafeMutablePointer<Int>?) -> Int32
@_silgen_name("unified_app_runner_take_requests")
func unified_app_runner_take_requests(_ vm: UnsafeMutableRawPointer?, _ out: UnsafeMutablePointer<UnsafePointer<UInt8>?>?, _ len: UnsafeMutablePointer<Int>?) -> Int32
@_silgen_name("unified_app_runner_last_error")
func unified_app_runner_last_error(_ vm: UnsafeMutableRawPointer?, _ out: UnsafeMutablePointer<UnsafePointer<UInt8>?>?, _ len: UnsafeMutablePointer<Int>?) -> Int32
@_silgen_name("unified_app_runner_suspend")
func unified_app_runner_suspend(_ vm: UnsafeMutableRawPointer?)
@_silgen_name("unified_app_runner_resume")
func unified_app_runner_resume(_ vm: UnsafeMutableRawPointer?)
@_silgen_name("unified_app_runner_focus")
func unified_app_runner_focus(_ vm: UnsafeMutableRawPointer?, _ focused: Int32)
@_silgen_name("unified_app_runner_modifiers")
func unified_app_runner_modifiers(_ vm: UnsafeMutableRawPointer?, _ bits: UInt32)
@_silgen_name("unified_app_runner_key")
func unified_app_runner_key(_ vm: UnsafeMutableRawPointer?, _ code: UInt32, _ pressed: Int32, _ repeated: Int32) -> Int32
@_silgen_name("unified_app_runner_text")
func unified_app_runner_text(_ vm: UnsafeMutableRawPointer?, _ target: UInt64, _ bytes: UnsafePointer<UInt8>?, _ len: Int) -> Int32
@_silgen_name("unified_app_runner_composition")
func unified_app_runner_composition(_ vm: UnsafeMutableRawPointer?, _ target: UInt64, _ bytes: UnsafePointer<UInt8>?, _ len: Int, _ start: Int, _ end: Int) -> Int32
@_silgen_name("unified_app_runner_clipboard")
func unified_app_runner_clipboard(_ vm: UnsafeMutableRawPointer?, _ request: UInt64, _ write: Int32, _ success: Int32, _ bytes: UnsafePointer<UInt8>?, _ len: Int) -> Int32
@_silgen_name("unified_app_runner_pointer_button")
func unified_app_runner_pointer_button(_ vm: UnsafeMutableRawPointer?, _ button: UInt16, _ pressed: Int32, _ x: Float, _ y: Float) -> Int32

@_silgen_name("unified_app_runner_replace_layer")
func unified_app_runner_replace_layer(_ vm: UnsafeMutableRawPointer?, _ layer: UnsafeMutableRawPointer?, _ width: UInt32, _ height: UInt32, _ scale: Float) -> Int32

@_silgen_name("unified_app_runner_text_snapshot")
func unified_app_runner_text_snapshot(_ vm: UnsafeMutableRawPointer?, _ out: UnsafeMutablePointer<UnsafePointer<UInt8>?>?, _ len: UnsafeMutablePointer<Int>?) -> Int32
@_silgen_name("unified_app_runner_text_edit")
func unified_app_runner_text_edit(_ vm: UnsafeMutableRawPointer?, _ target: UInt64, _ revision: UInt64, _ kind: UInt32, _ start: Int, _ end: Int, _ bytes: UnsafePointer<UInt8>?, _ len: Int, _ cursorStart: Int, _ cursorEnd: Int) -> Int32

@_silgen_name("unified_app_runner_text_query")
func unified_app_runner_text_query(_ vm: UnsafeMutableRawPointer?, _ target: UInt64, _ revision: UInt64, _ kind: UInt32, _ start: Int, _ end: Int, _ x: Float, _ y: Float, _ out: UnsafeMutablePointer<UnsafePointer<UInt8>?>?, _ len: UnsafeMutablePointer<Int>?) -> Int32

#if os(macOS)
import AppKit
#else
import UIKit
#endif

struct NativeHostState: Decodable {
    let title: String
    let cursor: String
    let cursorVisible: Bool
    let textInput: NativeTextInput?
    let presentations: UInt64
    let sessionId: String?
    let tickSequence: UInt64
    let frameCount: UInt64
    let readbackBytes: UInt64
}
private struct NativeHostRequest: Decodable {
    let kind: String
    let request: UInt64?
    let text: String?
    let message: String?
}

/// The host retains the backing layer until Rust destroys its surface. Every
/// callback, including destruction, runs serially on the main/creation thread.
@MainActor
final class SceneVMHandle: NativeTextHost {
    private var vm: UnsafeMutableRawPointer?
    private var layer: CAMetalLayer
    private var scale: CGFloat
    private(set) var state: NativeHostState?
    var nativeInputState: NativeTextInput? { state?.textInput }
    var onState: ((NativeHostState) -> Void)?
    var onError: ((String) -> Void)?
    private var reportedError: String?
    private var sourceSnapshot: NativeTextSnapshot?

    private static func pixels(_ value: CGFloat) -> UInt32 {
        value.isFinite ? UInt32(min(max(value.rounded(), 0), CGFloat(UInt32.max))) : 0
    }
    init?(layer: CAMetalLayer, size: CGSize, scale: CGFloat) {
        self.layer = layer
        self.scale = scale
        let ptr = Unmanaged.passUnretained(layer).toOpaque()
        let w = Self.pixels(size.width * scale)
        let h = Self.pixels(size.height * scale)
        guard let handle = unified_app_runner_create(ptr, w, h, Float(scale)) else { return nil }
        vm = handle
        refresh()
    }
    // Explicit teardown is called by the owning view before releasing its layer.
    func close() {
        guard let handle = vm else { return }
        unified_app_runner_destroy(handle)
        vm = nil
    }
    isolated deinit {
        // SceneVMHandle is main-actor owned by its view/document. The explicit
        // close path normally consumed this handle already.
        if let vm { unified_app_runner_destroy(vm) }
    }
    private func read(_ body: (UnsafeMutablePointer<UnsafePointer<UInt8>?>, UnsafeMutablePointer<Int>) -> Int32) -> Data? {
        var bytes: UnsafePointer<UInt8>?
        var count = 0
        guard body(&bytes, &count) == 0, count >= 0 else { return nil }
        guard let bytes else { return count == 0 ? Data() : nil }
        defer { unified_app_runner_free_data(bytes, count) }
        return Data(bytes: bytes, count: count)
    }
    private func reportError() {
        guard let vm, let data = read({ unified_app_runner_last_error(vm, $0, $1) }),
              let message = String(data: data, encoding: .utf8), !message.isEmpty else { return }
        if message != reportedError { reportedError = message; onError?(message) }
    }
    private func refreshState() {
        guard let vm else { return }
        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase
        if let data = read({ unified_app_runner_host_state(vm, $0, $1) }),
           let state = try? decoder.decode(NativeHostState.self, from: data) {
            let previous = self.state
            if sourceSnapshot?.target != state.textInput?.target { sourceSnapshot = nil }
            self.state = state
            if previous?.title != state.title || previous?.cursor != state.cursor || previous?.cursorVisible != state.cursorVisible || previous?.textInput != state.textInput { onState?(state) }
        }
    }
    private func refresh() {
        guard let vm else { return }
        refreshState()
        let decoder = JSONDecoder()
        var completedClipboard = false
        if let data = read({ unified_app_runner_take_requests(vm, $0, $1) }),
           let requests = try? decoder.decode([NativeHostRequest].self, from: data) {
            for request in requests {
                switch request.kind {
                case "clipboard_read":
                    guard let id = request.request else { continue }
                    #if os(macOS)
                    let value = NSPasteboard.general.string(forType: .string)
                    #else
                    let value = UIPasteboard.general.string
                    #endif
                    completeClipboard(id, write: false, text: value, success: value != nil)
                    completedClipboard = true
                case "clipboard_write":
                    guard let text = request.text else { continue }
                    #if os(macOS)
                    NSPasteboard.general.clearContents()
                    let success = NSPasteboard.general.setString(text, forType: .string)
                    #else
                    UIPasteboard.general.string = text
                    let success = true
                    #endif
                    if let id = request.request { completeClipboard(id, write: true, text: nil, success: success); completedClipboard = true }
                case "error": onError?(request.message ?? "Unable to update the player")
                default: break
                }
            }
        }
        // A completion can change text/selection before the next display tick.
        // Publish its new source epoch now so the next native key is not stale.
        if completedClipboard { refreshState() }
    }
    private func completeClipboard(_ id: UInt64, write: Bool, text: String?, success: Bool) {
        guard let vm else { return }
        let bytes = Array((text ?? "").utf8)
        bytes.withUnsafeBufferPointer {
            _ = unified_app_runner_clipboard(vm, id, write ? 1 : 0, success ? 1 : 0, $0.baseAddress, $0.count)
        }
    }
    /// Source text is queried on demand, separately from frame/host metrics.
    /// Selection-only changes can reuse the same immutable string/source epoch.
    func nativeTextSnapshot() -> NativeTextSnapshot? {
        guard let vm, let input = state?.textInput else { sourceSnapshot = nil; return nil }
        if var cached = sourceSnapshot, cached.target == input.target, cached.revision == input.revision {
            if let selection = input.selection { cached.selection = selection }
            sourceSnapshot = cached
            return cached
        }
        guard let data = read({ unified_app_runner_text_snapshot(vm, $0, $1) }) else { reportError(); return nil }
        sourceSnapshot = try? JSONDecoder().decode(NativeTextSnapshot.self, from: data)
        return sourceSnapshot
    }
    func queryNativeText(source: NativeTextSnapshot, range: NSRange?, point: CGPoint?) -> NativeTextAnswer? {
        guard let vm else { return nil }
        var start = -1, end = -1
        if let range {
            guard range.location >= 0, range.location != NSNotFound, range.length >= 0 else { return nil }
            let sum = range.location.addingReportingOverflow(range.length)
            guard !sum.overflow else { return nil }
            start = range.location; end = sum.partialValue
        }
        guard let data = read({ unified_app_runner_text_query(vm, source.target, source.revision, point == nil ? 0 : 1, start, end, Float(point?.x ?? 0), Float(point?.y ?? 0), $0, $1) }) else { return nil }
        let decoder = JSONDecoder(); decoder.keyDecodingStrategy = .convertFromSnakeCase
        return try? decoder.decode(NativeTextAnswer.self, from: data)
    }
    func navigateNativeText(source: NativeTextSnapshot, position: Int, direction: Int, count: Int) -> Int? {
        guard let vm, (0...3).contains(direction), count >= 0 else { return nil }
        guard let data = read({ unified_app_runner_text_query(vm, source.target, source.revision, UInt32(direction + 2), position, count, 0, 0, $0, $1) }) else { return nil }
        return (try? JSONDecoder().decode(NativeTextAnswer.self, from: data))?.position
    }
    @discardableResult
    func editNativeText(kind: UInt32, source: NativeTextSnapshot, range: NSRange?, text: String, selection: NSRange?) -> Bool {
        guard let vm else { return false }
        func endpoints(_ range: NSRange?) -> (Int, Int)? {
            guard let range, range.location != NSNotFound else { return (-1, -1) }
            guard range.location >= 0, range.length >= 0 else { return nil }
            let end = range.location.addingReportingOverflow(range.length)
            return end.overflow ? nil : (range.location, end.partialValue)
        }
        guard var range = endpoints(range), let cursor = endpoints(selection) else { return false }
        if (kind == 5 || kind == 6) && range.0 == -1 && source.selection.count == 2 { range = (source.selection[0], source.selection[1]) }
        let result = Array(text.utf8).withUnsafeBufferPointer {
            unified_app_runner_text_edit(vm, source.target, source.revision, kind, range.0, range.1, $0.baseAddress, $0.count, cursor.0, cursor.1)
        }
        if result < 0 { reportError() }
        refresh()
        return result > 0
    }
    func synchronizeHost() { if let state { onState?(state) } }
    func loadProject(data: Data) -> Bool {
        guard let vm, !data.isEmpty else { return false }
        let result = data.withUnsafeBytes {
            unified_app_runner_load_project(vm, $0.bindMemory(to: UInt8.self).baseAddress, $0.count)
        }
        if result < 0 { reportError() } else { reportedError = nil }
        refresh()
        return result == 0
    }
    func saveProject() -> Data? {
        guard let vm else { return nil }
        return read { unified_app_runner_save_project(vm, $0, $1) }
    }
    /// Keep the existing CPU session when Swift recreates its view/layer. The
    /// previous layer remains strongly retained until Rust drops its surface.
    func replaceLayer(_ next: CAMetalLayer, size: CGSize, scale: CGFloat) -> Bool {
        guard let vm else { return false }
        let result = unified_app_runner_replace_layer(vm, Unmanaged.passUnretained(next).toOpaque(), Self.pixels(size.width * scale), Self.pixels(size.height * scale), Float(scale))
        guard result == 0 else { reportError(); return false }
        layer = next
        self.scale = scale
        refresh()
        return true
    }
    func resize(to size: CGSize, scale: CGFloat) {
        guard let vm else { return }
        self.scale = scale
        unified_app_runner_resize(vm, Self.pixels(size.width * scale), Self.pixels(size.height * scale), Float(scale))
        refresh()
    }
    func render() {
        guard let vm else { return }
        if unified_app_runner_render(vm) < 0 { reportError() }
        refresh()
    }
    func suspend() { if let vm { unified_app_runner_suspend(vm) }; refresh() }
    func resume() { if let vm { unified_app_runner_resume(vm) }; refresh() }
    func focus(_ focused: Bool) { if let vm { unified_app_runner_focus(vm, focused ? 1 : 0) }; refresh() }
    func modifiers(_ bits: UInt32) { if let vm { unified_app_runner_modifiers(vm, bits) } }
    @discardableResult
    func key(_ code: UInt32, pressed: Bool, repeated: Bool = false) -> Bool {
        guard let vm else { return false }
        let result = unified_app_runner_key(vm, code, pressed ? 1 : 0, repeated ? 1 : 0)
        refresh()
        return result > 0
    }
    func text(_ text: String, target: UInt64) {
        guard let vm, target != 0 else { return }
        Array(text.utf8).withUnsafeBufferPointer { _ = unified_app_runner_text(vm, target, $0.baseAddress, $0.count) }
        refresh()
    }
    func composition(_ text: String, selection: NSRange?, target: UInt64) {
        guard let vm, target != 0 else { return }
        var start = -1, end = -1
        if let selection, let range = Range(selection, in: text) {
            start = text[..<range.lowerBound].utf8.count
            end = text[..<range.upperBound].utf8.count
        }
        Array(text.utf8).withUnsafeBufferPointer { _ = unified_app_runner_composition(vm, target, $0.baseAddress, $0.count, start, end) }
        refresh()
    }
    func mouseDown(x: CGFloat, y: CGFloat, button: UInt16 = 0) { pointer(x, y, button, true) }
    func mouseUp(x: CGFloat, y: CGFloat, button: UInt16 = 0) { pointer(x, y, button, false) }
    private func pointer(_ x: CGFloat, _ y: CGFloat, _ button: UInt16, _ pressed: Bool) {
        if let vm { _ = unified_app_runner_pointer_button(vm, button, pressed ? 1 : 0, Float(x), Float(y)) }
        refresh()
    }
    func mouseMove(x: CGFloat, y: CGFloat) { if let vm { unified_app_runner_mouse_move(vm, Float(x), Float(y)) }; refresh() }
    func mouseMotion(dx: CGFloat, dy: CGFloat) { if let vm { unified_app_runner_mouse_motion(vm, Float(dx), Float(dy)) }; refresh() }
    func mouseLeave() { if let vm { unified_app_runner_mouse_leave(vm) }; refresh() }
    func scroll(dx: CGFloat, dy: CGFloat) { if let vm { unified_app_runner_scroll(vm, Float(dx), Float(dy)) }; refresh() }
    func pinch(scale: CGFloat, center: CGPoint) { if let vm { unified_app_runner_pinch(vm, Float(scale), Float(center.x), Float(center.y)) }; refresh() }
}
