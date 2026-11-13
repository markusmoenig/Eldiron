// Link against the real Creator static library; main-thread foreign-layer check.
import AppKit
import QuartzCore

@main
@MainActor
struct NativeCreatorHostCheck {
    static func state(_ host: UInt64) -> [String: Any] {
        guard let text = eldiron_creator_state_v1(host) else { fatalError("Missing Creator host") }
        defer { rust_free_string(text) }
        return try! JSONSerialization.jsonObject(with: Data(String(cString: text).utf8)) as! [String: Any]
    }
    static func pump(_ host: UInt64, _ seconds: Double) {
        let until = Date().addingTimeInterval(seconds)
        repeat {
            precondition(eldiron_creator_render_v1(host) >= 0, "Creator presentation failed")
            if let requests = eldiron_creator_requests_v1(host) { rust_free_string(requests) }
            RunLoop.current.run(until: Date().addingTimeInterval(0.02))
        } while Date() < until
    }
    static func reply(_ pointer: UnsafeMutablePointer<CChar>?) -> [String: Any] {
        guard let pointer else { fatalError("Invalid host") }
        defer { rust_free_string(pointer) }
        return try! JSONSerialization.jsonObject(with: Data(String(cString: pointer).utf8)) as! [String: Any]
    }
    static func value(_ pointer: UnsafeMutablePointer<CChar>?) -> [String: Any] {
        let response = reply(pointer)
        precondition(response["ok"] as? Bool == true, "Document request failed: \(response)")
        return response["value"] as! [String: Any]
    }
    static func checkDocuments(_ host: UInt64) {
        // Toolbar/menu commands use host requests rather than native filesystem dialogs.
        for code: UInt32 in [1, 3, 4] { precondition(eldiron_creator_command_v1(host, code)) }
        let _ = value(eldiron_creator_document_info_v1(host))
        _ = eldiron_creator_render_v1(host)
        guard let requests = eldiron_creator_requests_v1(host) else { fatalError("Missing host requests") }
        let actions = try! JSONSerialization.jsonObject(with: Data(String(cString: requests).utf8)) as! [[String: Any]]
        rust_free_string(requests)
        for kind in ["open_document", "save_document", "save_document_as"] {
            precondition(actions.contains { $0["kind"] as? String == kind && $0["document"] is String })
        }
        let initial = value(eldiron_creator_document_info_v1(host))
        let document = initial["document"] as! String
        let invalid = document.withCString { reply(eldiron_creator_document_load_v1(host, $0, nil, 0, nil, false)) }
        precondition(invalid["ok"] as? Bool == false)
        precondition(value(eldiron_creator_document_info_v1(host))["document"] as? String == document)
        let save = document.withCString { value(eldiron_creator_document_prepare_save_v1(host, $0)) }
        let ticket = (save["ticket"] as! NSNumber).uint64Value
        let length = (save["length"] as! NSNumber).intValue
        precondition(length > 0)
        precondition(eldiron_creator_document_copy_save_v1(host, ticket, nil, 0) == length)
        var sentinel: UInt8 = 73
        precondition(eldiron_creator_document_copy_save_v1(host, ticket, &sentinel, 1) == length)
        precondition(sentinel == 73, "Undersized copy may not write partial bytes")
        precondition(reply(eldiron_creator_document_finish_save_v1(host, ticket + 1, true, nil))["ok"] as? Bool == false)
        var data = Data(count: length)
        let copied = data.withUnsafeMutableBytes { bytes in
            eldiron_creator_document_copy_save_v1(host, ticket, bytes.bindMemory(to: UInt8.self).baseAddress, UInt(bytes.count))
        }
        precondition(copied == length)
        let url = URL(fileURLWithPath: "/private/tmp/eldiron-swift-host-document.eldiron")
        try! data.write(to: url, options: .atomic)
        let completed = url.path.withCString { value(eldiron_creator_document_finish_save_v1(host, ticket, true, $0)) }
        precondition(completed["saved"] as? Bool == true && completed["clean"] as? Bool == true)
        precondition(value(eldiron_creator_document_info_v1(host))["dirty"] as? Bool == false)
        precondition(reply(eldiron_creator_document_finish_save_v1(host, ticket, true, nil))["ok"] as? Bool == false)
        precondition(eldiron_creator_document_copy_save_v1(host, ticket, nil, 0) == 0)
        let disk = try! Data(contentsOf: url)
        precondition(disk == data)
        let loaded = document.withCString { expected in disk.withUnsafeBytes { bytes in
            value(eldiron_creator_document_load_v1(host, expected, bytes.bindMemory(to: UInt8.self).baseAddress, UInt(bytes.count), nil, false))
        } }
        precondition(loaded["document"] as? String != document)
        precondition(document.withCString { reply(eldiron_creator_document_prepare_save_v1(host, $0))["ok"] as? Bool } == false)
        precondition((state(host)["readback_bytes"] as! NSNumber).uint64Value == 0)
    }
    static func checkImageClipboard(_ host: UInt64) {
        // A disposable copy of the shipped source gives the board one known
        // searchable bitmap. No system clipboard or native responder is used.
        var root = URL(fileURLWithPath: #filePath)
        for _ in 0..<4 { root.deleteLastPathComponent() }
        let fixture = try! Data(contentsOf: root.appendingPathComponent("test_projects/3DStarter.eldiron"))
        var project = try! JSONSerialization.jsonObject(with: fixture) as! [String: Any]
        var tiles = project["tiles"] as! [String: Any]
        let tileID = "a41cf692-e03b-4579-a8e9-3d77972d1858"
        var tile = tiles[tileID] as! [String: Any]
        tile["alias"] = "Clipboard source"
        tiles[tileID] = tile
        project["tiles"] = tiles
        let bytes = try! JSONSerialization.data(withJSONObject: project)
        let document = value(eldiron_creator_document_info_v1(host))["document"] as! String
        _ = document.withCString { expected in bytes.withUnsafeBytes { data in
            value(eldiron_creator_document_load_v1(host, expected, data.bindMemory(to: UInt8.self).baseAddress, UInt(data.count), nil, false))
        } }
        precondition(eldiron_creator_resize_v1(host, 2000, 1073, 1.25))
        pump(host, 0.3)
        func click(_ x: Float, _ y: Float) {
            _ = eldiron_creator_hover_v1(host, x, y)
            _ = eldiron_creator_touch_down_v1(host, x, y)
            _ = eldiron_creator_touch_up_v1(host, x, y)
            pump(host, 0.1)
        }
        click(120, 640)
        let source = reply(eldiron_creator_text_snapshot_v1(host))
        let text = Data("Clipboard source".utf8)
        precondition(text.withUnsafeBytes { data in
            eldiron_creator_text_edit_v1(host, (source["target"] as! NSNumber).uint64Value,
                (source["revision"] as! NSNumber).uint64Value, 1, 0,
                (source["text"] as! String).utf16.count,
                data.bindMemory(to: UInt8.self).baseAddress, UInt(data.count), -1, -1)
        } == 1)
        _ = eldiron_creator_key_v1(host, 0x100, true, false, nil)
        _ = eldiron_creator_key_v1(host, 0x100, false, false, nil)
        pump(host, 0.2)
        click(142, 706)
        _ = eldiron_creator_modifiers_v1(host, false, false, false, true)
        "[".withCString { text in
            _ = eldiron_creator_key_v1(host, 0, true, false, text)
            _ = eldiron_creator_key_v1(host, 0, false, false, text)
        }
        pump(host, 0.2)
        _ = eldiron_creator_modifiers_v1(host, false, false, false, false)
        click(25, 310)
        click(390, 166) // Focus the RGBA view outside the editable source.
        precondition(eldiron_creator_edit_command_v1(host, 1))
        func imageRequest(_ kind: String) -> [String: Any] {
            // Input can precede the next scheduled application update. Drain
            // after pumping it; a single immediate render may be idle.
            let deadline = Date().addingTimeInterval(10)
            var commands: [[String: Any]] = []
            repeat {
                _ = eldiron_creator_render_v1(host)
                if let requests = eldiron_creator_requests_v1(host) {
                    commands += try! JSONSerialization.jsonObject(with: Data(String(cString: requests).utf8)) as! [[String: Any]]
                    rust_free_string(requests)
                }
                if let command = commands.first(where: { $0["kind"] as? String == kind }) { return command }
                RunLoop.current.run(until: Date().addingTimeInterval(0.02))
            } while Date() < deadline
            fatalError("Missing \(kind): \(commands); \(state(host))")
        }
        let command = imageRequest("image_write")
        let request = (command["request"] as! NSNumber).uint64Value
        let length = (command["length"] as! NSNumber).intValue
        precondition(length > 0 && eldiron_creator_clipboard_image_copy_v1(host, request, nil, 0) == length)
        var sentinel: UInt8 = 73
        precondition(eldiron_creator_clipboard_image_copy_v1(host, request, &sentinel, 1) == length && sentinel == 73)
        precondition(eldiron_creator_clipboard_image_copy_v1(host, request + 1, nil, 0) == 0)
        var png = Data(count: length)
        precondition(png.withUnsafeMutableBytes { data in
            eldiron_creator_clipboard_image_copy_v1(host, request, data.bindMemory(to: UInt8.self).baseAddress, UInt(data.count))
        } == length)
        guard let image = NSBitmapImageRep(data: png) else { fatalError("Invalid clipboard PNG") }
        precondition(image.pixelsWide == 16 && image.pixelsHigh == 16)
        precondition(eldiron_creator_clipboard_image_complete_v1(host, request, true, false, nil, 0))
        precondition(eldiron_creator_clipboard_image_copy_v1(host, request, nil, 0) == 0)
        precondition(!eldiron_creator_clipboard_image_complete_v1(host, request, true, true, nil, 0))
        precondition(eldiron_creator_edit_command_v1(host, 2))
        let read = (imageRequest("image_read")["request"] as! NSNumber).uint64Value
        precondition(!eldiron_creator_clipboard_image_complete_v1(host, read, false, true, nil, 1))
        precondition(png.withUnsafeBytes { data in
            let pointer = data.bindMemory(to: UInt8.self).baseAddress
            precondition(!eldiron_creator_clipboard_image_complete_v1(host, read, false, true, pointer, 64 * 1024 * 1024 + 1))
            return eldiron_creator_clipboard_image_complete_v1(host, read, false, true, pointer, UInt(data.count))
        })

        precondition((state(host)["root_cpu_bytes"] as! NSNumber).uint64Value == 0)
        precondition((state(host)["readback_bytes"] as! NSNumber).uint64Value == 0)
    }
    static func main() {
        setbuf(stdout, nil)
        _ = NSApplication.shared
        precondition(eldiron_creator_abi_version() == 1)
        precondition(eldiron_creator_create_v1(1, nil, 1200, 800, 1) == 0)
        let layer = CAMetalLayer()
        let pointer = Unmanaged.passUnretained(layer).toOpaque()
        precondition(eldiron_creator_create_v1(2, pointer, 1200, 800, 1) == 0)
        let host = eldiron_creator_create_v1(1, pointer, 1200, 800, 1)
        precondition(host != 0)
        precondition(eldiron_creator_create_v1(1, pointer, 1200, 800, 1) == 0, "Cannot replace an existing document by initializing twice")
        defer { _ = eldiron_creator_destroy_v1(host) }
        pump(host, 0.5)
        var snapshot = state(host)
        precondition((snapshot["presentations"] as! NSNumber).uint64Value > 0)
        precondition((snapshot["root_cpu_bytes"] as! NSNumber).uint64Value == 0)
        precondition((snapshot["readback_bytes"] as! NSNumber).uint64Value == 0)
        let frames = (snapshot["presentations"] as! NSNumber).uint64Value
        let updates = (snapshot["updates"] as! NSNumber).uint64Value
        _ = eldiron_creator_suspend_v1(host)
        pump(host, 0.1)
        precondition((state(host)["presentations"] as! NSNumber).uint64Value == frames)
        precondition((state(host)["updates"] as! NSNumber).uint64Value > updates,
                     "Presentation suppression must not stop CPU application updates")
        _ = eldiron_creator_resume_v1(host)
        pump(host, 0.1)
        let beforeZeroSize = (state(host)["presentations"] as! NSNumber).uint64Value
        _ = eldiron_creator_resize_v1(host, 0, 0, 1)
        pump(host, 0.1)
        precondition((state(host)["presentations"] as! NSNumber).uint64Value == beforeZeroSize)
        _ = eldiron_creator_resize_v1(host, 1440, 960, 1.5)
        pump(host, 0.1)
        snapshot = state(host)
        precondition((snapshot["logical_size"] as! [Double]) == [960, 640])
        let replacement = CAMetalLayer()
        precondition(eldiron_creator_replace_layer_v1(host, Unmanaged.passUnretained(replacement).toOpaque()))
        pump(host, 0.1)
        precondition((state(host)["presentations"] as! NSNumber).uint64Value > frames)
        precondition((state(host)["readback_bytes"] as! NSNumber).uint64Value == 0)
        // Keep both backing layers alive through surface destruction.
        checkDocuments(host)
        checkImageClipboard(host)
        precondition(eldiron_creator_destroy_v1(host))
        precondition(!eldiron_creator_destroy_v1(host))
        precondition(eldiron_creator_render_v1(host) == -1)
        precondition(eldiron_creator_state_v1(host) == nil)
        // Recreate on the same retained layer and reject every old generation.
        let next = eldiron_creator_create_v1(1, pointer, 1200, 800, 1)
        precondition(next > host)
        precondition(!eldiron_creator_resize_v1(host, 1, 1, 1))
        precondition(!eldiron_creator_clipboard_complete_v1(host, 1, false, true, nil))
        precondition(eldiron_creator_clipboard_image_copy_v1(host, 1, nil, 0) == 0)
        precondition(!eldiron_creator_clipboard_image_complete_v1(host, 1, false, true, nil, 0))
        precondition(eldiron_creator_document_info_v1(host) == nil)
        precondition(eldiron_creator_document_copy_save_v1(host, 1, nil, 0) == 0)
        pump(next, 0.1)
        precondition(eldiron_creator_destroy_v1(next))
        withExtendedLifetime((layer, replacement)) {}
        print("Native Creator CAMetalLayer/handle/document/PNG transactions passed")
    }
}
