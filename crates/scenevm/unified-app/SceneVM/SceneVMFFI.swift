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

/// Thin Swift wrapper around the SceneVM FFI for CAMetalLayer presentation.
final class SceneVMHandle {
    private var vm: UnsafeMutableRawPointer?
    private weak var layer: CAMetalLayer?
    private var scale: CGFloat

    init?(layer: CAMetalLayer, size: CGSize, scale: CGFloat) {
        self.scale = scale
        let ptr = Unmanaged.passUnretained(layer).toOpaque()
        let w = UInt32(max(max(layer.drawableSize.width, size.width * scale), 1))
        let h = UInt32(max(max(layer.drawableSize.height, size.height * scale), 1))
        guard let handle = unified_app_runner_create(ptr, w, h, Float(scale)) else {
            return nil
        }
        self.layer = layer
        self.vm = handle
    }

    func resize(to size: CGSize, scale: CGFloat) {
        guard let vm else { return }
        self.scale = scale
        let drawable = layer?.drawableSize ?? CGSize(width: size.width * scale, height: size.height * scale)
        let w = UInt32(max(drawable.width, 1))
        let h = UInt32(max(drawable.height, 1))
        unified_app_runner_resize(vm, w, h, Float(scale))
    }

    func render() {
        guard let vm else { return }
        _ = unified_app_runner_render(vm)
    }

    func mouseDown(x: CGFloat, y: CGFloat) {
        guard let vm else { return }
        unified_app_runner_mouse_down(vm, Float(x), Float(y))
    }

    func mouseUp(x: CGFloat, y: CGFloat) {
        guard let vm else { return }
        unified_app_runner_mouse_up(vm, Float(x), Float(y))
    }

    func mouseMove(x: CGFloat, y: CGFloat) {
        guard let vm else { return }
        unified_app_runner_mouse_move(vm, Float(x), Float(y))
    }

    func scroll(dx: CGFloat, dy: CGFloat) {
        guard let vm else { return }
        unified_app_runner_scroll(vm, Float(dx), Float(dy))
    }

    func pinch(scale: CGFloat, center: CGPoint) {
        guard let vm else { return }
        unified_app_runner_pinch(vm, Float(scale), Float(center.x), Float(center.y))
    }

    func setTheme(isDark: Bool, size: CGSize) {
        guard let vm else { return }
        let w = UInt32(max(size.width * scale, 1))
        let h = UInt32(max(size.height * scale, 1))
        unified_app_runner_set_theme(vm, isDark ? 1 : 0, w, h)
    }

    // MARK: - Undo/Redo Support

    /// Perform undo operation
    /// Returns true if undo was performed, false if nothing to undo
    func undo() -> Bool {
        guard let vm else { return false }
        return unified_app_runner_undo(vm) > 0
    }

    /// Perform redo operation
    /// Returns true if redo was performed, false if nothing to redo
    func redo() -> Bool {
        guard let vm else { return false }
        return unified_app_runner_redo(vm) > 0
    }

    /// Check if undo is available
    func canUndo() -> Bool {
        guard let vm else { return false }
        return unified_app_runner_can_undo(vm) > 0
    }

    /// Check if redo is available
    func canRedo() -> Bool {
        guard let vm else { return false }
        return unified_app_runner_can_redo(vm) > 0
    }

    /// Get description of next undo action (e.g., "Undo Change Slider")
    func undoDescription() -> String? {
        guard let vm else { return nil }

        var strPtr: UnsafePointer<UInt8>? = nil
        var strLen: Int = 0

        let result = unified_app_runner_undo_description(vm, &strPtr, &strLen)

        guard result == 0, let ptr = strPtr, strLen > 0 else {
            return nil
        }

        defer {
            unified_app_runner_free_string(ptr, strLen)
        }

        let data = Data(bytes: ptr, count: strLen)
        return String(data: data, encoding: .utf8)
    }

    /// Get description of next redo action (e.g., "Redo Change Slider")
    func redoDescription() -> String? {
        guard let vm else { return nil }

        var strPtr: UnsafePointer<UInt8>? = nil
        var strLen: Int = 0

        let result = unified_app_runner_redo_description(vm, &strPtr, &strLen)

        guard result == 0, let ptr = strPtr, strLen > 0 else {
            return nil
        }

        defer {
            unified_app_runner_free_string(ptr, strLen)
        }

        let data = Data(bytes: ptr, count: strLen)
        return String(data: data, encoding: .utf8)
    }

    deinit {
        if let vm {
            unified_app_runner_destroy(vm)
        }
    }

    // MARK: - Project Management

    /// Save the current project state to JSON string
    /// Returns nil if save fails or app doesn't implement save_project
    func saveProject() -> String? {
        guard let vm else { return nil }

        var jsonPtr: UnsafePointer<UInt8>? = nil
        var jsonLen: Int = 0

        let result = unified_app_runner_save_project(vm, &jsonPtr, &jsonLen)

        guard result == 0, let ptr = jsonPtr, jsonLen > 0 else {
            return nil
        }

        defer {
            unified_app_runner_free_json(ptr, jsonLen)
        }

        let data = Data(bytes: ptr, count: jsonLen)
        return String(data: data, encoding: .utf8)
    }

    /// Load project state from JSON string
    /// Returns true if load was successful
    func loadProject(json: String) -> Bool {
        guard let vm else { return false }

        guard let data = json.data(using: .utf8) else {
            return false
        }

        return data.withUnsafeBytes { (buffer: UnsafeRawBufferPointer) -> Bool in
            guard let baseAddress = buffer.baseAddress else {
                return false
            }

            let ptr = baseAddress.assumingMemoryBound(to: UInt8.self)
            let result = unified_app_runner_load_project(vm, ptr, buffer.count)
            return result == 0
        }
    }

    /// Check if the app has unsaved changes
    func hasUnsavedChanges() -> Bool {
        guard let vm else { return false }
        return unified_app_runner_has_unsaved_changes(vm) > 0
    }

    /// Export project data in the specified format
    /// Returns exported data as Data, or nil if export fails
    func exportData(format: String) -> Data? {
        guard let vm else { return nil }
        guard let formatData = format.data(using: .utf8) else { return nil }

        var dataPtr: UnsafePointer<UInt8>? = nil
        var dataLen: Int = 0

        let result = formatData.withUnsafeBytes { (buffer: UnsafeRawBufferPointer) -> Int32 in
            guard let baseAddress = buffer.baseAddress else {
                return -2
            }
            let ptr = baseAddress.assumingMemoryBound(to: UInt8.self)
            return unified_app_runner_export_data(vm, ptr, buffer.count, &dataPtr, &dataLen)
        }

        guard result == 0, let ptr = dataPtr, dataLen > 0 else {
            return nil
        }

        defer {
            unified_app_runner_free_data(ptr, dataLen)
        }

        return Data(bytes: ptr, count: dataLen)
    }

    /// Import data into the project
    /// Returns true if import was successful
    func importData(_ data: Data, fileType: String) -> Bool {
        guard let vm else { return false }
        guard let fileTypeData = fileType.data(using: .utf8) else { return false }

        return data.withUnsafeBytes { (dataBuffer: UnsafeRawBufferPointer) -> Bool in
            guard let dataAddress = dataBuffer.baseAddress else {
                return false
            }

            return fileTypeData.withUnsafeBytes { (typeBuffer: UnsafeRawBufferPointer) -> Bool in
                guard let typeAddress = typeBuffer.baseAddress else {
                    return false
                }

                let dataPtr = dataAddress.assumingMemoryBound(to: UInt8.self)
                let typePtr = typeAddress.assumingMemoryBound(to: UInt8.self)
                let result = unified_app_runner_import_data(vm, dataPtr, dataBuffer.count, typePtr, typeBuffer.count)
                return result == 0
            }
        }
    }
}
