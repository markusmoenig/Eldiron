#if os(macOS)
import AppKit

@MainActor
private final class RecordedTextHost: NativeTextHost {
    struct Edit {
        let kind: UInt32
        let source: NativeTextSnapshot
        let range: NSRange?
        let text: String
        let selection: NSRange?
    }
    var snapshot = NativeTextSnapshot(target: 41, revision: 1, text: "A👩Z", selection: [3,3], marked: nil)
    var next: NativeTextSnapshot?
    var edits: [Edit] = []
    var nativeInputState: NativeTextInput? {
        NativeTextInput(target: snapshot.target, revision: snapshot.revision, selection: snapshot.selection, area: [8,12,2,24])
    }
    func nativeTextSnapshot() -> NativeTextSnapshot? { snapshot }
    func navigateNativeText(source: NativeTextSnapshot, position: Int, direction: Int, count: Int) -> Int? { nil }
    func queryNativeText(source: NativeTextSnapshot, range: NSRange?, point: CGPoint?) -> NativeTextAnswer? { nil }
    func editNativeText(kind: UInt32, source: NativeTextSnapshot, range: NSRange?, text: String, selection: NSRange?) -> Bool {
        // This stub records the adapter's operations. Shared editor policy,
        // Unicode conversion and undo are exercised by the actual Rust tests.
        edits.append(Edit(kind: kind, source: source, range: range, text: text, selection: selection))
        if let next { snapshot = next; self.next = nil }
        return true
    }
}
@MainActor
func checkNativeTextClient() {
    func key(_ text: String, flags: NSEvent.ModifierFlags = [], code: UInt16 = 0) -> NSEvent {
        NSEvent.keyEvent(with: .keyDown, location: .zero, modifierFlags: flags,
                        timestamp: 0, windowNumber: 0, context: nil,
                        characters: text, charactersIgnoringModifiers: text,
                        isARepeat: false, keyCode: code)!
    }
    for event in [key("a"), key("A", flags: .shift), key("e", flags: .option), key(" ", code: 49)] {
        precondition(NativeTextClient.shouldInterpret(event, consumed: true, hasTextClient: true),
                     "A focused editor's consumed printable key still needs native text interpretation")
        precondition(!NativeTextClient.shouldInterpret(event, consumed: true, hasTextClient: false),
                     "Consumed game controls must not become text")
    }
    for event in [key("a", flags: .command), key("a", flags: .control),
                  key("\r", code: 36), key("\u{7f}", code: 51), key("\u{f702}", code: 123)] {
        precondition(!NativeTextClient.shouldInterpret(event, consumed: true, hasTextClient: true),
                     "Handled shortcuts, deletion and navigation must not execute twice")
    }
    precondition(NativeTextClient.shouldInterpret(key("a"), consumed: false, hasTextClient: true))
    let host = RecordedTextHost()
    let view = NSView(frame: NSRect(x: 0, y: 0, width: 320, height: 120))
    var commands = 0
    var client: NativeTextClient? = NativeTextClient(target: 41, host: host, view: view) { _ in commands += 1 }
    weak var retained = client
    var context: NativeTextContext? = NativeTextContext(client: client!)
    client = nil
    precondition(retained != nil, "The context must retain its explicitly unowned native client")
    let active = retained!
    var actual = NSRange(location: NSNotFound, length: 0)
    let entire = active.attributedSubstring(forProposedRange: NSRange(location: 0, length: 4), actualRange: &actual)
    precondition(entire?.string == "A👩Z" && actual == NSRange(location: 0, length: 4))
    precondition(active.attributedSubstring(forProposedRange: NSRange(location: 1, length: 1), actualRange: nil) == nil)
    precondition(active.selectedRange() == NSRange(location: 3, length: 0))
    host.next = NativeTextSnapshot(target: 41, revision: 2, text: "A日本Z", selection: [3,3], marked: [1,3])
    active.setMarkedText("日本", selectedRange: NSRange(location: 2, length: 0), replacementRange: NSRange(location: 1, length: 2))
    let compose = host.edits.last!
    precondition(compose.kind == 2 && compose.source.revision == 1 && compose.range == NSRange(location: 1, length: 2))
    precondition(compose.text == "日本" && compose.selection == NSRange(location: 2, length: 0))
    precondition(active.markedRange() == NSRange(location: 1, length: 2))
    host.next = NativeTextSnapshot(target: 41, revision: 3, text: "A日本Z", selection: [3,3], marked: nil)
    active.unmarkText()
    precondition(host.edits.last!.kind == 3 && host.edits.last!.source.revision == 2)
    precondition(!active.hasMarkedText())
    active.beginKey()
    active.insertText("!", replacementRange: NSRange(location: NSNotFound, length: 0))
    active.endKey()
    precondition(host.edits.last!.kind == 5 && host.edits.last!.text == "!")
    let count = host.edits.count
    host.snapshot = NativeTextSnapshot(target: 99, revision: 7, text: "Other field", selection: [0,0], marked: nil)
    active.insertText("stale", replacementRange: NSRange(location: NSNotFound, length: 0))
    active.setMarkedText("stale", selectedRange: NSRange(location: 0, length: 0), replacementRange: NSRange(location: NSNotFound, length: 0))
    active.doCommand(by: #selector(NSResponder.moveLeft(_:)))
    precondition(host.edits.count == count && commands == 0)
    precondition(active.selectedRange().location == NSNotFound)
    context?.deactivate(); context = nil
    retained = nil
    print("Native macOS text client: surrounding UTF-16 text, marked commit, typed policy, retained context client and stale-field callbacks passed")
}
#endif
