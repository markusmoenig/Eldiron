//
//  SceneVMApp.swift
//  SceneVM Unified Template
//
//  Created by Markus Moenig on 10/12/25.
//

import SwiftUI

@main
struct SceneVMApp: App {
    var body: some Scene {
        // Both macOS and iOS use the same closure-based syntax
        DocumentGroup(newDocument: { SceneVMDocument() }) { file in
            DocumentView(document: file.document)
                #if os(iOS)
                .ignoresSafeArea()
                #endif
        }
        #if os(macOS)
        .commands {
            // Add Undo/Redo commands to Edit menu
            CommandGroup(after: .undoRedo) {
                Button("Undo") {
                    performUndo(for: file.document)
                }
                .keyboardShortcut("z", modifiers: .command)
                .disabled(!canUndo(for: file.document))

                Button("Redo") {
                    performRedo(for: file.document)
                }
                .keyboardShortcut("z", modifiers: [.command, .shift])
                .disabled(!canRedo(for: file.document))
            }
        }
        #endif
    }

    #if os(macOS)
    private func performUndo(for document: SceneVMDocument) {
        guard let handle = document.sceneVMHandle else { return }
        _ = handle.undo()
    }

    private func performRedo(for document: SceneVMDocument) {
        guard let handle = document.sceneVMHandle else { return }
        _ = handle.redo()
    }

    private func canUndo(for document: SceneVMDocument) -> Bool {
        guard let handle = document.sceneVMHandle else { return false }
        return handle.canUndo()
    }

    private func canRedo(for document: SceneVMDocument) -> Bool {
        guard let handle = document.sceneVMHandle else { return false }
        return handle.canRedo()
    }
    #endif
}
