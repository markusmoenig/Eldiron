//
//  SceneVMDocument.swift
//  SceneVM Unified Template
//
//  Document-based model for SceneVM projects
//  File extension: .scenevm
//

import SwiftUI
import Combine
import UniformTypeIdentifiers

// Define custom UTI for SceneVM documents (.scenevm)
extension UTType {
    static let scenevm = UTType(exportedAs: "com.scenevm.project")
}

/// SceneVM Document class for both macOS and iOS
final class SceneVMDocument: ReferenceFileDocument, ObservableObject {
    /// The SceneVM handle for this document
    var sceneVMHandle: SceneVMHandle?
    
    /// The project JSON data (in-memory representation)
    var projectJSON: String = "{}"
    
    // MARK: - ReferenceFileDocument Conformance
    
    static var readableContentTypes: [UTType] { [.scenevm] }
    
    required init(configuration: ReadConfiguration) throws {
        guard let data = configuration.file.regularFileContents else {
            throw CocoaError(.fileReadCorruptFile)
        }
        
        guard let jsonString = String(data: data, encoding: .utf8) else {
            throw CocoaError(.fileReadInapplicableStringEncoding)
        }
        
        self.projectJSON = jsonString
    }
    
    init() {
        // Create empty project
        self.projectJSON = "{\"version\":\"1.0\"}"
    }
    
    func snapshot(contentType: UTType) throws -> Data {
        guard let handle = sceneVMHandle else {
            // Return current JSON if SceneVM not ready yet
            guard let data = projectJSON.data(using: .utf8) else {
                throw CocoaError(.fileWriteInapplicableStringEncoding)
            }
            return data
        }
        
        guard let jsonString = handle.saveProject() else {
            throw CocoaError(.coderInvalidValue)
        }
        
        guard let data = jsonString.data(using: .utf8) else {
            throw CocoaError(.fileWriteInapplicableStringEncoding)
        }
        
        self.projectJSON = jsonString
        return data
    }
    
    func fileWrapper(snapshot: Data, configuration: WriteConfiguration) throws -> FileWrapper {
        return FileWrapper(regularFileWithContents: snapshot)
    }
}

// MARK: - SwiftUI Document View

struct DocumentView: View {
    @ObservedObject var document: SceneVMDocument
    
    var body: some View {
        SceneVMView(document: document)
            #if !os(macOS)
            .ignoresSafeArea()
            #endif
    }
}
