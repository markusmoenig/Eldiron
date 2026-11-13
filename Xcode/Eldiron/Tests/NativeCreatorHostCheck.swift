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
    static func main() {
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
        precondition(eldiron_creator_destroy_v1(host))
        precondition(!eldiron_creator_destroy_v1(host))
        precondition(eldiron_creator_render_v1(host) == -1)
        precondition(eldiron_creator_state_v1(host) == nil)
        // Recreate on the same retained layer and reject every old generation.
        let next = eldiron_creator_create_v1(1, pointer, 1200, 800, 1)
        precondition(next > host)
        precondition(!eldiron_creator_resize_v1(host, 1, 1, 1))
        precondition(!eldiron_creator_clipboard_complete_v1(host, 1, false, true, nil))
        precondition(eldiron_creator_document_info_v1(host) == nil)
        precondition(eldiron_creator_document_copy_save_v1(host, 1, nil, 0) == 0)
        pump(next, 0.1)
        precondition(eldiron_creator_destroy_v1(next))
        withExtendedLifetime((layer, replacement)) {}
        print("Native Creator CAMetalLayer/handle/document transactions passed")
    }
}
