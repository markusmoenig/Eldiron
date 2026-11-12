import SwiftUI

@main
struct SceneVMApp: App {
    var body: some Scene {
        DocumentGroup(newDocument: { SceneVMDocument() }) { file in
            DocumentView(document: file.document)
        }
    }
}
