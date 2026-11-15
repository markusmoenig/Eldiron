//
//  GameViewController.swift
//  Xcode2Rust macOS
//
//  Created by Markus Moenig on 16/10/22.
//

import Cocoa
import MetalKit

// Our macOS specific view controller
class GameViewController: NSViewController, NSWindowDelegate {

    var renderer: Renderer!
    var mtkView:  RMTKView!
    private var closeApproval = CreatorCloseApproval()

    var hasUnapprovedChanges: Bool {
        guard let renderer else { return false }
        return closeApproval.requiresConfirmation(host: renderer.handle, hasChanges: renderer.hasUnsavedChanges)
    }
    func windowDidBecomeKey(_ notification: Notification) {
        closeApproval.reset()
    }


    override func viewDidLoad() {
        super.viewDidLoad()

        guard let mtkView = self.view as? RMTKView else {
            print("View attached to GameViewController is not an MTKView")
            return
        }

        // Select the device to render with.  We choose the default device
        guard let defaultDevice = MTLCreateSystemDefaultDevice() else {
            print("Metal is not supported on this device")
            return
        }

        let minimumWidthConstraint = NSLayoutConstraint(item: view,
                                                                 attribute: .width,
                                                                 relatedBy: .greaterThanOrEqual,
                                                                 toItem: nil,
                                                                 attribute: .notAnAttribute,
                                                                 multiplier: 1,
                                                                 constant: 1300)

        let minimumHeightConstraint = NSLayoutConstraint(item: view,
                                                                 attribute: .height,
                                                                 relatedBy: .greaterThanOrEqual,
                                                                 toItem: nil,
                                                                 attribute: .notAnAttribute,
                                                                 multiplier: 1,
                                                                 constant: 700)
        NSLayoutConstraint.activate([
            minimumWidthConstraint,
            minimumHeightConstraint
        ])

        mtkView.device = defaultDevice

        renderer = Renderer(metalKitView: mtkView)
        mtkView.renderer = renderer

        renderer.mtkView(mtkView, drawableSizeWillChange: mtkView.drawableSize)

        mtkView.delegate = renderer
    }

    override func viewDidAppear() {
        super.viewDidAppear()
        closeApproval.reset()
        if let window = view.window {
            window.delegate = self
            let autosaveName = "EldironCreatorMainWindow"
            window.setFrameAutosaveName(autosaveName)
            _ = window.setFrameUsingName(autosaveName)
        }
    }

    func windowWillClose(_ notification: Notification) {
        renderer?.close()
    }

    func windowShouldClose(_ sender: NSWindow) -> Bool {
        closeApproval.reset()
        if renderer?.hasUnsavedChanges != true {
            return true
        }

        let alert = NSAlert()
        alert.messageText = "Unsaved Changes"
        alert.informativeText = "You have unsaved changes. Close this window?"
        alert.alertStyle = .warning
        alert.addButton(withTitle: "Close Window")
        alert.addButton(withTitle: "Cancel")
        let shouldClose = alert.runModal() == .alertFirstButtonReturn
        if shouldClose {
            // Last-window termination can reuse this decision for this host only.
            closeApproval.approve(host: renderer.handle)
        }
        return shouldClose
    }

    @IBAction func undo_menu(_ sender: NSMenuItem) {
        renderer?.editCommand(3)
        renderer?.needsUpdate()
    }

    @IBAction func open_menu(_ sender: NSMenuItem) {
        renderer?.openDocument()
        renderer?.needsUpdate()
    }

    @IBAction func save_menu(_ sender: NSMenuItem) {
        renderer?.saveDocument()
        renderer?.needsUpdate()
    }

    @IBAction func save_as_menu(_ sender: NSMenuItem) {
        renderer?.saveDocument(as: true)
        renderer?.needsUpdate()
    }

    @IBAction func redo_menu(_ sender: NSMenuItem) {
        renderer?.editCommand(4)
        renderer?.needsUpdate()
    }

    @IBAction func cut_menu(_ sender: NSMenuItem) {
        renderer?.editCommand(0)
        renderer?.needsUpdate()
    }

    @IBAction func copy_menu(_ sender: NSMenuItem) {
        renderer?.editCommand(1)
        renderer?.needsUpdate()
    }

    @IBAction func paste_menu(_ sender: NSMenuItem) {
        renderer?.editCommand(2)
        renderer?.needsUpdate()
    }
}
