// Swift owns the view/layer and platform services. Rust owns WGPU presentation.
import MetalKit
import QuartzCore
import UniformTypeIdentifiers

@MainActor
final class Renderer: NSObject, MTKViewDelegate {
    unowned let view: RMTKView
    private var timer: Timer?
    private var observers: [NSObjectProtocol] = []
    private(set) var handle: UInt64 = 0
    private var ready: Bool { handle != 0 }
    private var failed = false
    private var hasChanges = false
    private(set) var nativeInputState: NativeTextInput?
    private var sourceSnapshot: NativeTextSnapshot?
    // Outlive Rust's surface, including while the NSView/UIView is torn down.
    private var backingLayer: CAMetalLayer?
    #if os(iOS)
    private enum MobileDocumentOperation {
        case open(host: UInt64, document: String)
        case save(host: UInt64, ticket: UInt64)
    }
    private var mobileDocument: MobileDocumentOperation?
    private var mobileExportURL: URL?
    #endif

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
        handle = eldiron_creator_create_v1(1, Unmanaged.passUnretained(layer).toOpaque(), size.0, size.1, density)
        guard ready else { NSLog("Could not initialize native Creator WGPU surface"); return }
        observeLifecycle()
        let timer = Timer(timeInterval: 1.0 / 60.0, repeats: true) { [weak self] _ in MainActor.assumeIsolated { self?.tick() } }
        self.timer = timer
        RunLoop.main.add(timer, forMode: .common)
    }

    deinit {
        timer?.invalidate()
        observers.forEach { NotificationCenter.default.removeObserver($0) }
        if handle != 0 { _ = eldiron_creator_destroy_v1(handle) }
        #if os(iOS)
        if let mobileExportURL { try? FileManager.default.removeItem(at: mobileExportURL.deletingLastPathComponent()) }
        #endif
    }

    func close() {
        timer?.invalidate()
        timer = nil
        observers.forEach { NotificationCenter.default.removeObserver($0) }
        observers.removeAll()
        if handle != 0 {
            let retired = handle
            handle = 0
            _ = eldiron_creator_destroy_v1(retired)
        }
        nativeInputState = nil
        sourceSnapshot = nil
        backingLayer = nil
    }

    private func physicalSize() -> (UInt32, UInt32) {
        let scale = CGFloat(density)
        return (UInt32(max(0, (view.bounds.width * scale).rounded())),
                UInt32(max(0, (view.bounds.height * scale).rounded())))
    }

    private func tick() {
        guard ready && !failed else { return }
        let size = physicalSize()
        eldiron_creator_resize_v1(handle, size.0, size.1, density)
        let result = eldiron_creator_render_v1(handle)
        if result == -2, let layer = view.layer as? CAMetalLayer {
            failed = !eldiron_creator_replace_layer_v1(handle, Unmanaged.passUnretained(layer).toOpaque())
            if !failed { backingLayer = layer }
        } else if result < 0 {
            failed = true
            NSLog("Native Creator presentation failed")
        }
        serviceRequests()
        guard ready else { return }
        refreshTextState()
        #if os(macOS)
        let changed = eldiron_creator_has_changes_v1(handle)
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
            observers.append(center.addObserver(forName: name, object: nil, queue: .main) { [weak self, handle] note in
                MainActor.assumeIsolated {
                    guard let self, let window = note.object as? NSWindow, window === self.view.window else { return }
                    _ = eldiron_creator_focus_v1(handle, focused)
                }
            })
        }
        for (name, active) in [(NSWindow.didMiniaturizeNotification, false), (NSWindow.didDeminiaturizeNotification, true)] {
            observers.append(center.addObserver(forName: name, object: nil, queue: .main) { [weak self, handle] note in
                MainActor.assumeIsolated {
                    guard let self, let window = note.object as? NSWindow, window === self.view.window else { return }
                    if active { _ = eldiron_creator_resume_v1(handle) } else { _ = eldiron_creator_suspend_v1(handle) }
                }
            })
        }
        #else
        observers.append(center.addObserver(forName: UIApplication.didEnterBackgroundNotification, object: nil, queue: .main) { [handle] _ in _ = eldiron_creator_suspend_v1(handle) })
        observers.append(center.addObserver(forName: UIApplication.willEnterForegroundNotification, object: nil, queue: .main) { [handle] _ in _ = eldiron_creator_resume_v1(handle) })
        #endif
    }

    private func serviceRequests() {
        guard let source = eldiron_creator_requests_v1(handle) else { return }
        defer { rust_free_string(source) }
        guard let data = String(cString: source).data(using: .utf8),
              let requests = try? JSONSerialization.jsonObject(with: data) as? [[String: Any]] else { return }
        for request in requests {
            let id = (request["request"] as? NSNumber)?.uint64Value
            switch request["kind"] as? String {
            #if os(macOS) || os(iOS)
            case "open_document": openDocument(expected: request["document"] as? String)
            case "save_document": saveDocument(expected: request["document"] as? String)
            case "save_document_as": saveDocument(as: true, expected: request["document"] as? String)
            #endif
            case "title":
                #if os(macOS)
                view.window?.title = request["title"] as? String ?? "Eldiron Creator"
                #endif
            case "error": NSLog("Creator: %@", request["message"] as? String ?? "Unknown error")
            case "exit":
                #if os(macOS)
                view.window?.performClose(nil)
                return
                #else
                eldiron_creator_suspend_v1(handle)
                #endif
            case "read":
                guard let id else { continue }
                #if os(macOS)
                let text = NSPasteboard.general.string(forType: .string)
                #else
                let text = UIPasteboard.general.string
                #endif
                if let text { _ = text.withCString { eldiron_creator_clipboard_complete_v1(handle, id, false, true, $0) } }
                else { eldiron_creator_clipboard_complete_v1(handle, id, false, false, nil) }
            case "write":
                guard let text = request["text"] as? String else { continue }
                #if os(macOS)
                NSPasteboard.general.clearContents()
                let success = NSPasteboard.general.setString(text, forType: .string)
                #else
                UIPasteboard.general.string = text
                let success = true
                #endif
                if let id { eldiron_creator_clipboard_complete_v1(handle, id, true, success, nil) }
            case "image_read":
                guard let id else { continue }
                #if os(macOS)
                let png = NSPasteboard.general.data(forType: .png) ?? NSImage(pasteboard: .general)?.tiffRepresentation.flatMap { NSBitmapImageRep(data: $0)?.representation(using: .png, properties: [:]) }
                #else
                let png = UIPasteboard.general.image?.pngData()
                #endif
                if let png, png.count <= 64 * 1024 * 1024 {
                    _ = png.withUnsafeBytes { buffer in
                        eldiron_creator_clipboard_image_complete_v1(handle, id, false, true, buffer.bindMemory(to: UInt8.self).baseAddress, UInt(png.count))
                    }
                } else {
                    _ = eldiron_creator_clipboard_image_complete_v1(handle, id, false, false, nil, 0)
                }
            case "image_write":
                guard let id else { continue }
                let count = eldiron_creator_clipboard_image_copy_v1(handle, id, nil, 0)
                guard count > 0, count <= 64 * 1024 * 1024 else {
                    _ = eldiron_creator_clipboard_image_complete_v1(handle, id, true, false, nil, 0)
                    continue
                }
                var png = Data(count: Int(count))
                let copied = png.withUnsafeMutableBytes { buffer in
                    eldiron_creator_clipboard_image_copy_v1(handle, id, buffer.bindMemory(to: UInt8.self).baseAddress, count)
                }
                var success = false
                if copied == count {
                    #if os(macOS)
                    NSPasteboard.general.clearContents()
                    success = NSPasteboard.general.setData(png, forType: .png)
                    #else
                    if let image = UIImage(data: png) {
                        UIPasteboard.general.image = image
                        success = true
                    }
                    #endif
                }
                _ = eldiron_creator_clipboard_image_complete_v1(handle, id, true, success, nil, 0)
            default: break
            }
        }
    }

    func draw(in view: MTKView) { tick() }
    func mtkView(_ view: MTKView, drawableSizeWillChange size: CGSize) {
        guard ready else { return }
        let size = physicalSize()
        eldiron_creator_resize_v1(handle, size.0, size.1, density)
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
        nativeInputState = decode(TextState.self, eldiron_creator_state_v1(handle))?.textInput
        view.synchronizeNativeText(host: self)
    }
    func nativeTextSnapshot() -> NativeTextSnapshot? {
        guard let input = nativeInputState else { sourceSnapshot = nil; return nil }
        if var cached = sourceSnapshot, cached.target == input.target, cached.revision == input.revision {
            if let selection = input.selection { cached.selection = selection }
            sourceSnapshot = cached
            return cached
        }
        sourceSnapshot = decode(NativeTextSnapshot.self, eldiron_creator_text_snapshot_v1(handle))
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
        return decode(NativeTextAnswer.self, eldiron_creator_text_query_v1(handle, source.target, source.revision,
            point == nil ? 0 : 1, range.0, range.1, Float(point?.x ?? 0), Float(point?.y ?? 0)))
    }
    func navigateNativeText(source: NativeTextSnapshot, position: Int, direction: Int, count: Int) -> Int? {
        guard (0...3).contains(direction), count >= 0 else { return nil }
        return decode(NativeTextAnswer.self, eldiron_creator_text_query_v1(handle, source.target, source.revision,
            UInt32(direction + 2), position, count, 0, 0))?.position
    }
    func editNativeText(kind: UInt32, source: NativeTextSnapshot, range: NSRange?, text: String, selection: NSRange?) -> Bool {
        guard var range = endpoints(range), let cursor = endpoints(selection) else { return false }
        if (kind == 5 || kind == 6) && range.0 == -1 && source.selection.count == 2 {
            range = (source.selection[0], source.selection[1])
        }
        let result = Array(text.utf8).withUnsafeBufferPointer {
            eldiron_creator_text_edit_v1(handle, source.target, source.revision, kind, range.0, range.1,
                $0.baseAddress, UInt($0.count), cursor.0, cursor.1)
        }
        refreshTextState()
        return result > 0
    }
}

// The platform owns I/O. Rust owns document activation and the addressed snapshot.
extension Renderer {
    private struct Document: Decodable {
        let document: String
        let path: String?
        let title: String
        let dirty: Bool
    }
    private struct Save: Decodable { let ticket: UInt64; let document: String; let length: Int }
    private struct Saved: Decodable { let document: String; let saved: Bool; let clean: Bool }
    private struct Reply<T: Decodable>: Decodable { let ok: Bool; let value: T?; let error: String? }
    private func response<T: Decodable>(_ type: T.Type, _ pointer: UnsafeMutablePointer<CChar>?) -> T? {
        guard let reply = decode(Reply<T>.self, pointer) else { return nil }
        if !reply.ok { NSLog("Creator document: %@", reply.error ?? "Request failed") }
        return reply.value
    }
    var hasUnsavedChanges: Bool { eldiron_creator_has_changes_v1(handle) }
    func command(_ code: UInt32) { _ = eldiron_creator_command_v1(handle, code); needsUpdate() }
    func editCommand(_ code: UInt32) { _ = eldiron_creator_edit_command_v1(handle, code); needsUpdate() }
    var theme: UInt32 { eldiron_creator_get_theme_v1(handle) }
    func setTheme(_ theme: UInt32) { _ = eldiron_creator_set_theme_v1(handle, theme); needsUpdate() }

    // Capture both the host and active document before a panel/read can yield.
    // A replacement host or tab cannot receive this completion accidentally.
    func loadDocument(from url: URL) {
        guard let document = response(Document.self, eldiron_creator_document_info_v1(handle)) else { return }
        readDocument(from: url, host: handle, document: document.document)
    }
    private func readDocument(from url: URL, host: UInt64, document: String) {
        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            let access = url.startAccessingSecurityScopedResource()
            defer { if access { url.stopAccessingSecurityScopedResource() } }
            let result = Result { try Data(contentsOf: url) }
            DispatchQueue.main.async { [weak self] in
                guard let self, self.handle == host else { return }
                switch result {
                case .success(let data):
                    let _: Document? = document.withCString { expected in
                        url.path.withCString { path in
                            data.withUnsafeBytes { bytes in
                                self.response(Document.self, eldiron_creator_document_load_v1(host, expected,
                                    bytes.bindMemory(to: UInt8.self).baseAddress, UInt(bytes.count), path, false))
                            }
                        }
                    }
                    self.needsUpdate()
                case .failure(let error): NSLog("Creator open: %@", error.localizedDescription)
                }
            }
        }
    }
    private func finishSave(host: UInt64, ticket: UInt64, success: Bool, url: URL?) {
        let pointer = url.map { url in url.path.withCString { eldiron_creator_document_finish_save_v1(host, ticket, success, $0) } }
            ?? eldiron_creator_document_finish_save_v1(host, ticket, success, nil)
        let _: Saved? = response(Saved.self, pointer)
        needsUpdate()
    }
    private func writeSave(_ save: Save, to requestedURL: URL, host: UInt64) {
        let url = requestedURL.pathExtension.lowercased() == "eldiron" ? requestedURL : requestedURL.appendingPathExtension("eldiron")
        guard save.length > 0 else { finishSave(host: host, ticket: save.ticket, success: false, url: nil); return }
        var data = Data(count: save.length)
        let copied = data.withUnsafeMutableBytes { bytes in
            eldiron_creator_document_copy_save_v1(host, save.ticket, bytes.bindMemory(to: UInt8.self).baseAddress, UInt(bytes.count))
        }
        guard copied == save.length else { finishSave(host: host, ticket: save.ticket, success: false, url: nil); return }
        DispatchQueue.global(qos: .userInitiated).async { [weak self, data] in
            let access = url.startAccessingSecurityScopedResource()
            defer { if access { url.stopAccessingSecurityScopedResource() } }
            let result = Result { try data.write(to: url, options: .atomic) }
            DispatchQueue.main.async { [weak self] in
                guard let self, self.handle == host else { return }
                switch result {
                case .success: self.finishSave(host: host, ticket: save.ticket, success: true, url: url)
                case .failure(let error):
                    NSLog("Creator save: %@", error.localizedDescription)
                    self.finishSave(host: host, ticket: save.ticket, success: false, url: nil)
                }
            }
        }
    }
    #if os(macOS)
    func openDocument(expected: String? = nil) {
        guard let document = response(Document.self, eldiron_creator_document_info_v1(handle)),
              expected == nil || expected == document.document else { return }
        let host = handle
        let panel = NSOpenPanel()
        panel.allowedContentTypes = [UTType(filenameExtension: "eldiron") ?? .data]
        panel.allowsMultipleSelection = false
        panel.canChooseDirectories = false
        panel.begin { [weak self] answer in
            guard answer == .OK, let self, self.handle == host, let url = panel.url else { return }
            self.readDocument(from: url, host: host, document: document.document)
        }
    }
    func saveDocument(as saveAs: Bool = false, expected: String? = nil) {
        guard let document = response(Document.self, eldiron_creator_document_info_v1(handle)),
              expected == nil || expected == document.document,
              let save = document.document.withCString({ response(Save.self, eldiron_creator_document_prepare_save_v1(handle, $0)) }) else { return }
        let host = handle
        if !saveAs, let path = document.path { writeSave(save, to: URL(fileURLWithPath: path), host: host); return }
        let panel = NSSavePanel()
        panel.allowedContentTypes = [UTType(filenameExtension: "eldiron") ?? .data]
        panel.nameFieldStringValue = document.path.map { URL(fileURLWithPath: $0).lastPathComponent } ?? "Untitled.eldiron"
        panel.begin { [weak self] answer in
            guard let self, self.handle == host else { return }
            guard answer == .OK, let url = panel.url else { self.finishSave(host: host, ticket: save.ticket, success: false, url: nil); return }
            self.writeSave(save, to: url, host: host)
        }
    }
    #endif
}

#if os(iOS)
extension Renderer: UIDocumentPickerDelegate {
    private func documentPresenter() -> UIViewController? {
        guard let controller = view.window?.rootViewController, controller.presentedViewController == nil else { return nil }
        return controller
    }
    func openDocument(expected: String? = nil) {
        guard mobileDocument == nil, let presenter = documentPresenter(),
              let document = response(Document.self, eldiron_creator_document_info_v1(handle)),
              expected == nil || expected == document.document else { return }
        mobileDocument = .open(host: handle, document: document.document)
        let picker = UIDocumentPickerViewController(forOpeningContentTypes: [UTType(filenameExtension: "eldiron") ?? .data], asCopy: false)
        picker.allowsMultipleSelection = false
        picker.delegate = self
        presenter.present(picker, animated: true)
    }
    func saveDocument(as saveAs: Bool = false, expected: String? = nil) {
        guard mobileDocument == nil, let presenter = documentPresenter(),
              let document = response(Document.self, eldiron_creator_document_info_v1(handle)),
              expected == nil || expected == document.document,
              let save = document.document.withCString({ response(Save.self, eldiron_creator_document_prepare_save_v1(handle, $0)) }) else { return }
        var data = Data(count: save.length)
        let copied = data.withUnsafeMutableBytes { bytes in
            eldiron_creator_document_copy_save_v1(handle, save.ticket, bytes.bindMemory(to: UInt8.self).baseAddress, UInt(bytes.count))
        }
        guard save.length > 0, copied == save.length else { finishSave(host: handle, ticket: save.ticket, success: false, url: nil); return }
        // UIDocumentPicker owns the external copy. Keep the exact immutable export
        // alive until its addressed delegate completion or cancellation.
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString, isDirectory: true)
        let url = directory.appendingPathComponent(document.path.map { URL(fileURLWithPath: $0).lastPathComponent } ?? "Untitled.eldiron")
        do {
            try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
            try data.write(to: url, options: .atomic)
        } catch {
            try? FileManager.default.removeItem(at: directory)
            NSLog("Creator export: %@", error.localizedDescription)
            finishSave(host: handle, ticket: save.ticket, success: false, url: nil)
            return
        }
        mobileExportURL = url
        mobileDocument = .save(host: handle, ticket: save.ticket)
        let picker = UIDocumentPickerViewController(forExporting: [url], asCopy: true)
        picker.delegate = self
        presenter.present(picker, animated: true)
    }
    private func retireMobileDocument() {
        mobileDocument = nil
        if let url = mobileExportURL { try? FileManager.default.removeItem(at: url.deletingLastPathComponent()) }
        mobileExportURL = nil
    }
    func documentPicker(_ controller: UIDocumentPickerViewController, didPickDocumentsAt urls: [URL]) {
        guard let operation = mobileDocument, let url = urls.first else { documentPickerWasCancelled(controller); return }
        defer { retireMobileDocument() }
        switch operation {
        case .open(let host, let document): readDocument(from: url, host: host, document: document)
        case .save(let host, let ticket): finishSave(host: host, ticket: ticket, success: true, url: url)
        }
    }
    func documentPickerWasCancelled(_ controller: UIDocumentPickerViewController) {
        if case .save(let host, let ticket) = mobileDocument { finishSave(host: host, ticket: ticket, success: false, url: nil) }
        retireMobileDocument()
    }
}
#endif
