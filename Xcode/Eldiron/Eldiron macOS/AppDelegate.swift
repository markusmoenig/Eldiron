//
//  AppDelegate.swift
//  Xcode2Rust macOS
//
//  Created by Markus Moenig on 16/10/22.
//

import Cocoa

@main
@MainActor
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

    private func ensureFileSeparatorBetweenCloseAndSave() {
        guard let fileMenu = menuItem(named: "File")?.submenu else {
            return
        }
        guard
            let closeIndex = fileMenu.items.firstIndex(where: { $0.title == "Close" }),
            let saveIndex = fileMenu.items.firstIndex(where: { $0.title == "Save…" }),
            closeIndex < saveIndex
        else {
            return
        }

        // If there is already a separator between Close and Save..., keep as-is.
        if closeIndex + 1 < saveIndex,
           fileMenu.items[(closeIndex + 1)...(saveIndex - 1)].contains(where: { $0.isSeparatorItem }) {
            return
        }

        fileMenu.insertItem(NSMenuItem.separator(), at: closeIndex + 1)
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
        removeItems(in: "File", titled: ["Open Recent", "Page Setup…", "Print…"])
        bindFileMenuItem("New", action: #selector(newMenu))
        bindFileMenuItem("Open…", action: #selector(openMenu))
        bindFileMenuItem("Close", action: #selector(closeMenu))
        bindFileMenuItem("Save…", action: #selector(saveMenu))
        bindFileMenuItem("Save As…", action: #selector(saveAsMenu))
        ensureFileSeparatorBetweenCloseAndSave()
        trimEditMenuAfterPaste()
        removeTopLevelMenus(["Format", "View"])
        configureViewMenu()
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
        let docsItem = NSMenuItem(title: "Eldiron Docs", action: #selector(openDocs), keyEquivalent: "?")
        docsItem.target = self
        helpMenu.addItem(docsItem)
        mainMenu.insertItem(helpRoot, at: oldHelpIndex)
        mainMenu.setSubmenu(helpMenu, for: helpRoot)
    }

    private func configureViewMenu() {
        guard let mainMenu = NSApp.mainMenu else { return }
        let root = NSMenuItem(title: "View", action: nil, keyEquivalent: "")
        let menu = NSMenu(title: "View")
        for (index, title) in ["Brown", "Black", "Light"].enumerated() {
            let item = NSMenuItem(title: title, action: #selector(selectCreatorTheme(_:)), keyEquivalent: "")
            item.target = self
            item.tag = index
            menu.addItem(item)
        }
        menu.autoenablesItems = false
        root.submenu = menu
        mainMenu.addItem(root)
        updateThemeMenu(menu, selected: Int(creatorRenderer?.theme ?? 0))
    }

    private func updateThemeMenu(_ menu: NSMenu, selected: Int) {
        for item in menu.items { item.state = item.tag == selected ? .on : .off }
    }

    @objc private func selectCreatorTheme(_ sender: NSMenuItem) {
        creatorRenderer?.setTheme(UInt32(sender.tag))
        if let menu = sender.menu { updateThemeMenu(menu, selected: sender.tag) }
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

        gameMenu.addItem(NSMenuItem.separator())

        let showRoot = NSMenuItem(title: "Show", action: nil, keyEquivalent: "")
        let showMenu = NSMenu(title: "Show")

        let projectSidebar = NSMenuItem(title: "Project Sidebar", action: #selector(showProjectSidebarMenu), keyEquivalent: "f")
        projectSidebar.keyEquivalentModifierMask = [.command, .shift]
        projectSidebar.target = self
        showMenu.addItem(projectSidebar)

        let actionsSidebar = NSMenuItem(title: "Actions Sidebar", action: #selector(showActionsSidebarMenu), keyEquivalent: "g")
        actionsSidebar.keyEquivalentModifierMask = [.command, .shift]
        actionsSidebar.target = self
        showMenu.addItem(actionsSidebar)

        let console = NSMenuItem(title: "Console", action: #selector(showConsoleMenu), keyEquivalent: "h")
        console.keyEquivalentModifierMask = [.command, .shift]
        console.target = self
        showMenu.addItem(console)

        let debug = NSMenuItem(title: "Debug", action: #selector(showDebugLogMenu), keyEquivalent: "j")
        debug.keyEquivalentModifierMask = [.command, .shift]
        debug.target = self
        showMenu.addItem(debug)

        let help = NSMenuItem(title: "Help", action: #selector(showHelpSidebarMenu), keyEquivalent: "k")
        help.keyEquivalentModifierMask = [.command, .shift]
        help.target = self
        showMenu.addItem(help)

        showMenu.addItem(NSMenuItem.separator())

        let settings = NSMenuItem(title: "Settings", action: #selector(showSettingsMenu), keyEquivalent: "")
        settings.target = self
        showMenu.addItem(settings)

        let rules = NSMenuItem(title: "Rules", action: #selector(showRulesMenu), keyEquivalent: "")
        rules.target = self
        showMenu.addItem(rules)

        let locales = NSMenuItem(title: "Locales", action: #selector(showLocalesMenu), keyEquivalent: "")
        locales.target = self
        showMenu.addItem(locales)

        let audioFx = NSMenuItem(title: "Audio FX", action: #selector(showAudioFxMenu), keyEquivalent: "")
        audioFx.target = self
        showMenu.addItem(audioFx)

        let authoring = NSMenuItem(title: "Authoring", action: #selector(showAuthoringMenu), keyEquivalent: "")
        authoring.target = self
        showMenu.addItem(authoring)

        gameMenu.addItem(showRoot)
        gameMenu.setSubmenu(showMenu, for: showRoot)

        let insertIndex = if let editIndex = mainMenu.items.firstIndex(where: { $0.title == "Edit" }) {
            editIndex + 1
        } else {
            mainMenu.items.count
        }
        mainMenu.insertItem(gameRoot, at: insertIndex)
        mainMenu.setSubmenu(gameMenu, for: gameRoot)
    }

    private var creatorRenderer: Renderer? {
        (NSApp.keyWindow?.contentViewController as? GameViewController)?.renderer
            ?? NSApp.windows.compactMap { ($0.contentViewController as? GameViewController)?.renderer }.first
    }

    @objc private func openMenu() {
        creatorRenderer?.openDocument()
    }

    @objc private func newMenu() {
        creatorRenderer?.command(0)
    }

    @objc private func saveMenu() {
        creatorRenderer?.saveDocument()
    }

    @objc private func closeMenu() {
        creatorRenderer?.command(2)
    }

    @objc private func saveAsMenu() {
        creatorRenderer?.saveDocument(as: true)
    }

    @objc private func playMenu() {
        creatorRenderer?.command(5)
    }

    @objc private func pauseMenu() {
        creatorRenderer?.command(6)
    }

    @objc private func stopMenu() {
        creatorRenderer?.command(7)
    }

    @objc private func showSettingsMenu() {
        creatorRenderer?.command(8)
    }

    @objc private func showRulesMenu() {
        creatorRenderer?.command(9)
    }

    @objc private func showLocalesMenu() {
        creatorRenderer?.command(10)
    }

    @objc private func showAudioFxMenu() {
        creatorRenderer?.command(11)
    }

    @objc private func showAuthoringMenu() {
        creatorRenderer?.command(12)
    }

    @objc private func showProjectSidebarMenu() {
        creatorRenderer?.command(13)
    }

    @objc private func showActionsSidebarMenu() {
        creatorRenderer?.command(14)
    }

    @objc private func showDebugLogMenu() {
        creatorRenderer?.command(15)
    }

    @objc private func showConsoleMenu() {
        creatorRenderer?.command(16)
    }

    @objc private func showHelpSidebarMenu() {
        creatorRenderer?.command(17)
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
        setMenuIcon("New", "doc.badge.plus")
        setMenuIcon("Open…", "folder")
        setMenuIcon("Close", "xmark.circle")
        setMenuIcon("Save…", "square.and.arrow.down")
        setMenuIcon("Save As…", "square.and.arrow.down.on.square")
        setMenuIcon("Undo", "arrow.uturn.backward")
        setMenuIcon("Redo", "arrow.uturn.forward")
        setMenuIcon("Cut", "scissors")
        setMenuIcon("Copy", "doc.on.doc")
        setMenuIcon("Paste", "doc.on.clipboard")
        setMenuIcon("Play", "play.fill")
        setMenuIcon("Pause", "pause.fill")
        setMenuIcon("Stop", "stop.fill")
        setMenuIcon("Settings", "gearshape")
        setMenuIcon("Rules", "scroll")
        setMenuIcon("Locales", "globe")
        setMenuIcon("Audio FX", "waveform")
        setMenuIcon("Project Sidebar", "folder")
        setMenuIcon("Actions Sidebar", "bolt")
        setMenuIcon("Console", "terminal")
        setMenuIcon("Debug", "exclamationmark.triangle")
        setMenuIcon("Help", "questionmark.circle")
        setMenuIcon("Eldiron Docs", "book")
    }

    private func confirmCloseIfNeeded() -> Bool {
        if creatorRenderer?.hasUnsavedChanges != true {
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
