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
                                                                 constant: 1068)

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
        view.window?.delegate = self
    }

    func windowShouldClose(_ sender: NSWindow) -> Bool {
        if !rust_has_changes() {
            return true
        }

        let alert = NSAlert()
        alert.messageText = "Unsaved Changes"
        alert.informativeText = "You have unsaved changes. Are you sure you want to quit?"
        alert.alertStyle = .warning
        alert.addButton(withTitle: "Quit")
        alert.addButton(withTitle: "Cancel")
        let shouldClose = alert.runModal() == .alertFirstButtonReturn
        if shouldClose {
            // Avoid a second prompt in applicationShouldTerminate for this close flow.
            AppDelegate.bypassUnsavedPromptOnce = true
        }
        return shouldClose
    }

    @IBAction func undo_menu(_ sender: NSMenuItem) {
        rust_undo()
        renderer.needsUpdate()
    }

    @IBAction func open_menu(_ sender: NSMenuItem) {
        rust_open()
        renderer.needsUpdate()
    }

    @IBAction func save_menu(_ sender: NSMenuItem) {
        rust_save()
        renderer.needsUpdate()
    }

    @IBAction func save_as_menu(_ sender: NSMenuItem) {
        rust_save_as()
        renderer.needsUpdate()
    }

    @IBAction func redo_menu(_ sender: NSMenuItem) {
        rust_redo()
        renderer.needsUpdate()
    }

    @IBAction func cut_menu(_ sender: NSMenuItem) {
        if let text = rust_cut() {
            let str = String(cString: text)
            let pasteboard = NSPasteboard.general
            pasteboard.declareTypes([.string], owner: nil)
            pasteboard.setString(str, forType: .string)
            renderer.updateOnce()
        }
    }

    @IBAction func copy_menu(_ sender: NSMenuItem) {
        if let text = rust_copy() {
            let str = String(cString: text)
            let pasteboard = NSPasteboard.general
            pasteboard.declareTypes([.string], owner: nil)
            pasteboard.setString(str, forType: .string)
        }
    }

    @IBAction func paste_menu(_ sender: NSMenuItem) {
        let item = NSPasteboard.general.pasteboardItems?.first
        if let item = item {
            if let str = item.string(forType: NSPasteboard.PasteboardType.string) {
                rust_paste(str)
                renderer.updateOnce()
            }
        }
    }
}
