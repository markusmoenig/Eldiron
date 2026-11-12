import SwiftUI
import Combine
import UniformTypeIdentifiers

extension UTType {
    nonisolated static let eldironProject = UTType(exportedAs: "com.moenig.eldiron.project", conformingTo: .data)
    nonisolated static let legacySceneVMProject = UTType(exportedAs: "com.scenevm.project", conformingTo: .data)
}

@MainActor
final class PlayerViewState: ObservableObject {
    var handle: SceneVMHandle?
    @Published var errorMessage: String?
    nonisolated init() {}
}

/// A player keeps authored bytes independent of the running session. Both legacy
/// JSON and ZIP Eldiron projects reach the existing Rust decoder unchanged.
nonisolated final class SceneVMDocument: ReferenceFileDocument, ObservableObject {
    let player = PlayerViewState()
    let projectData: Data
    @MainActor var sceneVMHandle: SceneVMHandle? {
        get { player.handle }
        set { player.handle = newValue }
    }
    @MainActor var errorMessage: String? {
        get { player.errorMessage }
        set { player.errorMessage = newValue }
    }
    static var readableContentTypes: [UTType] { [.eldironProject, .legacySceneVMProject] }
    convenience init(configuration: ReadConfiguration) throws {
        guard let data = configuration.file.regularFileContents else { throw CocoaError(.fileReadCorruptFile) }
        self.init(data: data)
    }
    init(data: Data) { projectData = data }
    init() {
        projectData = Bundle.main.url(forResource: "game", withExtension: "eldiron")
            .flatMap { try? Data(contentsOf: $0) } ?? Data()
    }
    func snapshot(contentType: UTType) throws -> Data {
        // Playing changes runtime maps, time and UI state, never this source.
        projectData
    }
    func fileWrapper(snapshot: Data, configuration: WriteConfiguration) throws -> FileWrapper {
        FileWrapper(regularFileWithContents: snapshot)
    }
}
struct DocumentView: View {
    @ObservedObject var document: SceneVMDocument
    @ObservedObject private var state: PlayerViewState
    init(document: SceneVMDocument) {
        self.document = document
        self.state = document.player
    }
    var body: some View {
        SceneVMView(document: document)
            .alert("Unable to play project", isPresented: Binding(
                get: { state.errorMessage != nil },
                set: { if !$0 { state.errorMessage = nil } }
            )) { Button("OK") { state.errorMessage = nil } }
            message: { Text(state.errorMessage ?? "") }
            #if !os(macOS)
            .ignoresSafeArea()
            #endif
    }
}
