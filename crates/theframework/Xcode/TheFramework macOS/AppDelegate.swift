//
//  AppDelegate.swift
//  Xcode2Rust macOS
//
//  Created by Markus Moenig on 16/10/22.
//

import Cocoa

@main
class AppDelegate: NSObject, NSApplicationDelegate {
    static var bypassUnsavedPromptOnce: Bool = false

    private func menuItem(named title: String) -> NSMenuItem? {
        NSApp.mainMenu?.items.first { $0.title == title }
    }

    private func removeItems(in menuTitle: String, titled titles: Set<String>) {
        guard let submenu = menuItem(named: menuTitle)?.submenu else {
            return
        }
        for item in submenu.items.reversed() where titles.contains(item.title) {
            submenu.removeItem(item)
        }
    }

    private func bindFileMenuItem(_ title: String, action: Selector) {
        guard let fileMenu = menuItem(named: "File")?.submenu else {
            return
        }
        guard let item = fileMenu.items.first(where: { $0.title == title }) else {
            return
        }
        item.target = self
        item.action = action
        item.isEnabled = true
    }

    private func trimEditMenuAfterPaste() {
        guard let submenu = menuItem(named: "Edit")?.submenu else {
            return
        }
        guard let pasteIndex = submenu.items.firstIndex(where: { $0.title == "Paste" }) else {
            return
        }
        if pasteIndex + 1 < submenu.items.count {
            for idx in stride(from: submenu.items.count - 1, through: pasteIndex + 1, by: -1) {
                submenu.removeItem(at: idx)
            }
        }
    }

    private func removeTopLevelMenus(_ titles: Set<String>) {
        guard let menu = NSApp.mainMenu else {
            return
        }
        for item in menu.items.reversed() where titles.contains(item.title) {
            menu.removeItem(item)
        }
    }

    private func configureNativeMenus() {
        removeItems(in: "File", titled: ["Close", "Open Recent", "Page Setup…", "Print…"])
        bindFileMenuItem("New", action: #selector(newMenu))
        bindFileMenuItem("Open…", action: #selector(openMenu))
        bindFileMenuItem("Save…", action: #selector(saveMenu))
        bindFileMenuItem("Save As…", action: #selector(saveAsMenu))
        trimEditMenuAfterPaste()
        removeTopLevelMenus(["Format", "View"])
        configureGameMenu()

        guard let mainMenu = NSApp.mainMenu else {
            return
        }
        guard let oldHelpIndex = mainMenu.items.firstIndex(where: { $0.title == "Help" }) else {
            return
        }

        mainMenu.removeItem(at: oldHelpIndex)

        let helpRoot = NSMenuItem(title: "Help", action: nil, keyEquivalent: "")
        let helpMenu = NSMenu(title: "Help")
        let docsItem = NSMenuItem(title: "TheFramework Docs", action: #selector(openDocs), keyEquivalent: "?")
        docsItem.target = self
        helpMenu.addItem(docsItem)
        mainMenu.insertItem(helpRoot, at: oldHelpIndex)
        mainMenu.setSubmenu(helpMenu, for: helpRoot)
    }

    private func configureGameMenu() {
        guard let mainMenu = NSApp.mainMenu else {
            return
        }

        if let existing = mainMenu.items.first(where: { $0.title == "Game" }) {
            mainMenu.removeItem(existing)
        }

        let gameRoot = NSMenuItem(title: "Game", action: nil, keyEquivalent: "")
        let gameMenu = NSMenu(title: "Game")

        let play = NSMenuItem(title: "Play", action: #selector(playMenu), keyEquivalent: "p")
        play.target = self
        gameMenu.addItem(play)

        let pause = NSMenuItem(title: "Pause", action: #selector(pauseMenu), keyEquivalent: "o")
        pause.target = self
        gameMenu.addItem(pause)

        let stop = NSMenuItem(title: "Stop", action: #selector(stopMenu), keyEquivalent: "p")
        stop.keyEquivalentModifierMask = [.command, .shift]
        stop.target = self
        gameMenu.addItem(stop)

        let insertIndex = if let editIndex = mainMenu.items.firstIndex(where: { $0.title == "Edit" }) {
            editIndex + 1
        } else {
            mainMenu.items.count
        }
        mainMenu.insertItem(gameRoot, at: insertIndex)
        mainMenu.setSubmenu(gameMenu, for: gameRoot)
    }

    @objc private func openMenu() {
        rust_open()
    }

    @objc private func newMenu() {
        rust_new()
    }

    @objc private func saveMenu() {
        rust_save()
    }

    @objc private func saveAsMenu() {
        rust_save_as()
    }

    @objc private func playMenu() {
        rust_play()
    }

    @objc private func pauseMenu() {
        rust_pause()
    }

    @objc private func stopMenu() {
        rust_stop()
    }

    @objc private func openDocs() {
        NSWorkspace.shared.open(URL(string: "https://eldiron.com/docs")!)
    }

    @available(macOS 11.0, *)
    private func setMenuIcon(_ title: String, _ systemName: String) {
        guard let menu = NSApp.mainMenu else {
            return
        }
        func walk(_ item: NSMenuItem) {
            if item.title == title {
                item.image = NSImage(systemSymbolName: systemName, accessibilityDescription: nil)
            }
            item.submenu?.items.forEach(walk)
        }
        menu.items.forEach(walk)
    }

    private func configureMenuIcons() {
        guard #available(macOS 11.0, *) else {
            return
        }
        setMenuIcon("Undo", "arrow.uturn.backward")
        setMenuIcon("Redo", "arrow.uturn.forward")
        setMenuIcon("Cut", "scissors")
        setMenuIcon("Copy", "doc.on.doc")
        setMenuIcon("Paste", "doc.on.clipboard")
    }

    private func confirmCloseIfNeeded() -> Bool {
        if !rust_has_changes() {
            return true
        }

        let alert = NSAlert()
        alert.messageText = "Unsaved Changes"
        alert.informativeText = "You have unsaved changes. Are you sure you want to quit?"
        alert.alertStyle = .warning
        alert.addButton(withTitle: "Quit")
        alert.addButton(withTitle: "Cancel")
        return alert.runModal() == .alertFirstButtonReturn
    }

    func applicationDidFinishLaunching(_ aNotification: Notification) {
        configureNativeMenus()
        configureMenuIcons()
    }

    func applicationWillTerminate(_ aNotification: Notification) {
        // Insert code here to tear down your application
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        return true
    }

    func applicationShouldTerminate(_ sender: NSApplication) -> NSApplication.TerminateReply {
        if AppDelegate.bypassUnsavedPromptOnce {
            AppDelegate.bypassUnsavedPromptOnce = false
            return .terminateNow
        }
        return confirmCloseIfNeeded() ? .terminateNow : .terminateCancel
    }

}
