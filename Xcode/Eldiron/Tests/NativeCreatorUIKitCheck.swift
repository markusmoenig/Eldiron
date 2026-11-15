#if os(iOS)
import UIKit
import MetalKit
import Darwin

@MainActor
private final class AcceptanceCreatorController: GameViewController {
    override func loadView() { view = RMTKView(frame: CGRect(x: 0, y: 0, width: 1100, height: 760)) }
}

@main
@MainActor
final class NativeCreatorUIKitApp: UIResponder, UIApplicationDelegate {
    func application(_ application: UIApplication,
                     configurationForConnecting session: UISceneSession,
                     options: UIScene.ConnectionOptions) -> UISceneConfiguration {
        let configuration = UISceneConfiguration(name: "Acceptance", sessionRole: session.role)
        configuration.delegateClass = NativeCreatorUIKitScene.self
        return configuration
    }
}

@MainActor
final class NativeCreatorUIKitScene: UIResponder, UIWindowSceneDelegate {
    var window: UIWindow?
    private var controller: AcceptanceCreatorController?
    private var retiredController: AcceptanceCreatorController?
    private var retiredHost: UInt64 = 0
    private var initialDocument = ""
    private var timer: Timer?
    private var started = Date()
    private var firstPresentations: UInt64 = 0
    private var savedURL: URL?

    private func reply(_ pointer: UnsafeMutablePointer<CChar>?) -> [String: Any]? {
        guard let pointer else { return nil }
        defer { rust_free_string(pointer) }
        return try? JSONSerialization.jsonObject(with: Data(String(cString: pointer).utf8)) as? [String: Any]
    }
    private func value(_ pointer: UnsafeMutablePointer<CChar>?) -> [String: Any]? {
        guard let response = reply(pointer), response["ok"] as? Bool == true else { return nil }
        return response["value"] as? [String: Any]
    }

    func scene(_ scene: UIScene, willConnectTo session: UISceneSession,
               options connectionOptions: UIScene.ConnectionOptions) {
        guard let scene = scene as? UIWindowScene else { fatalError("No UIWindowScene") }
        checkNativeUIKitTextClient()
        let window = UIWindow(windowScene: scene)
        self.window = window
        installController(source: Bundle.main.url(forResource: "game", withExtension: "eldiron")!)
        started = Date()
        timer = Timer(timeInterval: 0.05, repeats: true) { [weak self] _ in
            MainActor.assumeIsolated { self?.advance() }
        }
        RunLoop.main.add(timer!, forMode: .common)
    }

    private func installController(source: URL) {
        let controller = AcceptanceCreatorController()
        self.controller = controller
        window!.rootViewController = controller
        window!.makeKeyAndVisible()
        controller.view.layoutIfNeeded()
        let host = controller.renderer.handle
        guard host != 0, let document = value(eldiron_creator_document_info_v1(host))?["document"] as? String else {
            finish(error: "Unable to create the actual UIKit Creator host"); return
        }
        initialDocument = document
        // Exercise the checked-in asynchronous Swift file-loading route.
        controller.renderer.loadDocument(from: source)
    }

    private func advance() {
        guard Date().timeIntervalSince(started) < 90 else {
            finish(error: "Timed out awaiting Creator document/presentation"); return
        }
        guard let controller, controller.renderer.handle != 0 else {
            finish(error: "Creator owner missing"); return
        }
        let host = controller.renderer.handle
        guard let document = value(eldiron_creator_document_info_v1(host)),
              let id = document["document"] as? String, id != initialDocument,
              let state = reply(eldiron_creator_state_v1(host)),
              let presentations = (state["presentations"] as? NSNumber)?.uint64Value,
              presentations >= 3 else { return }
        guard (state["root_cpu_bytes"] as? NSNumber)?.uint64Value == 0,
              (state["readback_bytes"] as? NSNumber)?.uint64Value == 0 else {
            finish(error: "Creator allocated root pixels or implicitly read back"); return
        }
        if firstPresentations == 0 {
            do {
                let save = id.withCString { value(eldiron_creator_document_prepare_save_v1(host, $0)) }!
                let ticket = (save["ticket"] as! NSNumber).uint64Value
                let length = (save["length"] as! NSNumber).intValue
                precondition(length > 0)
                var bytes = Data(count: length)
                let copied = bytes.withUnsafeMutableBytes { storage in
                    eldiron_creator_document_copy_save_v1(host, ticket,
                        storage.bindMemory(to: UInt8.self).baseAddress, UInt(storage.count))
                }
                precondition(copied == length)
                let url = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
                    .appendingPathComponent("creator-saved.eldiron")
                try bytes.write(to: url, options: .atomic)
                let disk = try Data(contentsOf: url)
                precondition(disk == bytes)
                let completed = url.path.withCString { value(eldiron_creator_document_finish_save_v1(host, ticket, true, $0)) }
                guard completed?["saved"] as? Bool == true, completed?["clean"] as? Bool == true else {
                    finish(error: "Creator save did not acknowledge its source document"); return
                }
                firstPresentations = presentations
                savedURL = url
                retiredHost = host
                retiredController = controller
                controller.renderer.close()
                guard controller.renderer.handle == 0, eldiron_creator_state_v1(host) == nil,
                      eldiron_creator_document_info_v1(host) == nil else {
                    finish(error: "Retained UIKit controller kept its closed Rust host alive"); return
                }
                installController(source: url)
            } catch { finish(error: error.localizedDescription) }
            return
        }
        guard host != retiredHost, eldiron_creator_state_v1(retiredHost) == nil,
              document["dirty"] as? Bool == false, savedURL != nil else {
            finish(error: "Creator reopen/stale-owner isolation failed"); return
        }
        finish(error: nil, presentations: presentations)
    }

    private func finish(error: String?, presentations: UInt64 = 0) {
        timer?.invalidate()
        timer = nil
        var result: [String: Any] = [
            "status": error == nil ? "passed" : "failed", "profile": "release",
            "uiKitTextProtocolPassed": true, "physicalIMEVerified": false,
            "firstPresentations": firstPresentations,
            "secondPresentations": presentations,
            "finalPresentations": firstPresentations + presentations,
            "readbackBytes": 0, "rootCPUBytes": 0,
            "elapsedSeconds": Date().timeIntervalSince(started)
        ]
        if let error { result["error"] = error }
        do {
            let url = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
                .appendingPathComponent("NativeUIKitAcceptance.json")
            try JSONSerialization.data(withJSONObject: result, options: [.prettyPrinted, .sortedKeys])
                .write(to: url, options: .atomic)
        } catch { NSLog("Unable to save Creator UIKit evidence: %@", error.localizedDescription) }
        controller?.renderer.close()
        withExtendedLifetime(retiredController) {}
        NSLog("Native Creator UIKit acceptance: %@", error ?? "passed")
        Darwin.exit(error == nil ? 0 : 1)
    }
}
#endif
