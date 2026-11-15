#if os(iOS)
import UIKit
import Darwin

// Optimized simulator application built by tools/check_native_uikit.py. This
// drives the checked-in MetalContainer, not a second presentation implementation.
@main
@MainActor
final class NativeUIKitAcceptanceApp: UIResponder, UIApplicationDelegate {
    func application(_ application: UIApplication,
                     configurationForConnecting session: UISceneSession,
                     options: UIScene.ConnectionOptions) -> UISceneConfiguration {
        let configuration = UISceneConfiguration(name: "Acceptance", sessionRole: session.role)
        configuration.delegateClass = NativeUIKitAcceptanceScene.self
        return configuration
    }
}

@MainActor
final class NativeUIKitAcceptanceScene: UIResponder, UIWindowSceneDelegate {
    var window: UIWindow?
    private var document: SceneVMDocument?
    private var container: MetalContainer?
    private var timer: Timer?
    private var first: NativeHostState?
    private var source = Data()
    private var started = Date()
    private var textClientPassed = false

    func scene(_ scene: UIScene, willConnectTo session: UISceneSession,
               options connectionOptions: UIScene.ConnectionOptions) {
        guard let scene = scene as? UIWindowScene else { fatalError("No UIWindowScene") }
        do {
            source = try Data(contentsOf: Bundle.main.url(forResource: "game", withExtension: "eldiron")!)
            checkNativeUIKitTextClient()
            textClientPassed = true
            let document = SceneVMDocument(data: source)
            self.document = document
            let window = UIWindow(windowScene: scene)
            window.rootViewController = UIViewController()
            self.window = window
            window.makeKeyAndVisible()
            installContainer()
            started = Date()
            timer = Timer(timeInterval: 0.05, repeats: true) { [weak self] _ in
                MainActor.assumeIsolated { self?.advance() }
            }
            RunLoop.main.add(timer!, forMode: .common)
        } catch { finish(error: error.localizedDescription) }
    }

    private func installContainer() {
        let root = window!.rootViewController!.view!
        root.layoutIfNeeded()
        var frame = root.bounds
        if first != nil { frame.size.width *= 0.75 }
        let view = MetalContainer(frame: frame)
        view.document = document
        root.addSubview(view)
        view.setNeedsLayout()
        view.layoutIfNeeded()
        container = view
    }

    private func advance() {
        guard Date().timeIntervalSince(started) < 60 else {
            finish(error: "Timed out awaiting native UIKit presentations")
            return
        }
        guard let document else { finish(error: "Lost document"); return }
        if let error = document.errorMessage { finish(error: error); return }
        guard let player = document.sceneVMHandle, let state = player.state,
              state.presentations >= 3, let session = state.sessionId else { return }
        guard state.readbackBytes == 0, player.saveProject() == source else {
            finish(error: "Readback or authored source preservation failed")
            return
        }
        if first == nil {
            let count = state.presentations
            let frames = state.frameCount
            player.resize(to: .zero, scale: 1.5)
            player.render()
            guard player.state!.presentations == count, player.state!.frameCount == frames else {
                finish(error: "Zero-size view advanced presentation"); return
            }
            player.suspend()
            player.render()
            guard player.state!.presentations == count else {
                finish(error: "Suspended view presented"); return
            }
            first = state
            // The document retains the exact CPU session while the real UIKit
            // view is dismantled, then supplies it to a different Metal layer.
            container?.close()
            container?.removeFromSuperview()
            container = nil
            installContainer()
            guard document.sceneVMHandle === player else {
                finish(error: "View recreation replaced the CPU session owner"); return
            }
            return
        }
        guard state.presentations > first!.presentations else { return }
        guard session == first!.sessionId, state.tickSequence >= first!.tickSequence,
              state.frameCount > first!.frameCount else {
            finish(error: "Session/timing lost across UIKit view recreation"); return
        }
        do {
            guard try document.snapshot(contentType: .eldironProject) == source else {
                finish(error: "Document snapshot persisted runtime state"); return
            }
        } catch { finish(error: error.localizedDescription); return }
        finish(error: nil, state: state)
    }

    private func finish(error: String?, state: NativeHostState? = nil) {
        timer?.invalidate()
        timer = nil
        var result: [String: Any] = [
            "status": error == nil ? "passed" : "failed",
            "profile": "release",
            "uiKitTextProtocolPassed": textClientPassed,
            "physicalIMEVerified": false,
            "firstPresentations": first?.presentations ?? 0,
            "finalPresentations": state?.presentations ?? 0,
            "readbackBytes": state?.readbackBytes ?? 0,
            "elapsedSeconds": Date().timeIntervalSince(started)
        ]
        if let error { result["error"] = error }
        do {
            let url = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
                .appendingPathComponent("NativeUIKitAcceptance.json")
            try JSONSerialization.data(withJSONObject: result, options: [.prettyPrinted, .sortedKeys])
                .write(to: url, options: .atomic)
        } catch { NSLog("Unable to save UIKit acceptance result: %@", error.localizedDescription) }
        container?.close()
        container?.removeFromSuperview()
        container = nil
        document?.sceneVMHandle?.close()
        document?.sceneVMHandle = nil
        NSLog("Native UIKit acceptance: %@", error ?? "passed")
        Darwin.exit(error == nil ? 0 : 1)
    }
}
#endif
