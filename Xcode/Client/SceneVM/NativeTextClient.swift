#if os(macOS)
import AppKit

/// One native input client per logical editor target. An old input context can
/// retain this object for delayed callbacks, but it can never edit a new field.
@MainActor
final class NativeTextContext: NSTextInputContext {
    // NSTextInputContext explicitly does not retain its client. Keeping it here
    // also makes any system-retained old context safe during delayed callbacks.
    private let retainedClient: any NSTextInputClient
    override init(client: any NSTextInputClient) {
        retainedClient = client
        super.init(client: client)
    }
}

@MainActor
final class NativeTextClient: NSObject, NSTextInputClient {
    /// Rust consumes keys addressed to an editor even when printable text still
    /// needs AppKit's keyboard layout/dead-key/IME interpretation. Commands and
    /// navigation already handled by Rust must not run a second time.
    static func shouldInterpret(_ event: NSEvent, consumed: Bool, hasTextClient: Bool) -> Bool {
        if !consumed { return true }
        guard hasTextClient,
              event.modifierFlags.intersection([.command, .control]).isEmpty,
              let characters = event.charactersIgnoringModifiers, !characters.isEmpty else { return false }
        return characters.unicodeScalars.allSatisfy {
            !CharacterSet.controlCharacters.contains($0) && !(0xF700...0xF8FF).contains($0.value)
        }
    }

    let target: UInt64
    private weak var host: (any NativeTextHost)?
    private weak var view: NSView?
    private let command: (Selector) -> Void
    private var keySource: NativeTextSnapshot?
    private var queriedSource: NativeTextSnapshot?
    private var compositionSource: NativeTextSnapshot?
    private var cancelling = false
    init(target: UInt64, host: any NativeTextHost, view: NSView, command: @escaping (Selector) -> Void) {
        self.target = target; self.host = host; self.view = view; self.command = command
    }
    private func source() -> NativeTextSnapshot? {
        guard let source = host?.nativeTextSnapshot(), source.target == target else { return nil }
        return source
    }
    private func query() -> NativeTextSnapshot? {
        let result = source()
        queriedSource = result
        return result
    }
    func beginKey() { keySource = source() }
    func endKey() { keySource = nil }
    func cancelForDeactivation() {
        guard !cancelling else { return }
        cancelling = true
        if let source = compositionSource ?? queriedSource ?? source() { _ = host?.editText(kind: 4, source: source) }
        compositionSource = nil; queriedSource = nil; keySource = nil
    }
    func hasMarkedText() -> Bool { source()?.marked != nil }
    func markedRange() -> NSRange { query()?.markedRange ?? NSRange(location: NSNotFound, length: 0) }
    func selectedRange() -> NSRange { query()?.selectedRange ?? NSRange(location: NSNotFound, length: 0) }
    func validAttributesForMarkedText() -> [NSAttributedString.Key] { [] }
    func setMarkedText(_ value: Any, selectedRange: NSRange, replacementRange: NSRange) {
        guard !cancelling, source() != nil, let source = compositionSource ?? keySource ?? queriedSource ?? source() else { return }
        let text = (value as? NSAttributedString)?.string ?? (value as? String ?? "")
        if host?.editText(kind: 2, source: source, range: replacementRange, text: text, selection: selectedRange) == true {
            compositionSource = self.source()
            queriedSource = compositionSource
        }
    }
    func insertText(_ value: Any, replacementRange: NSRange) {
        guard !cancelling, source() != nil, let source = compositionSource ?? keySource ?? queriedSource ?? source() else { return }
        let text = (value as? NSAttributedString)?.string ?? (value as? String ?? "")
        if replacementRange.location != NSNotFound {
            _ = host?.editText(kind: 1, source: source, range: replacementRange, text: text)
        } else {
            _ = host?.editText(kind: 5, source: source, text: text)
        }
        compositionSource = nil
        queriedSource = self.source()
    }
    func unmarkText() {
        guard let source = compositionSource ?? queriedSource ?? source() else { return }
        _ = host?.editText(kind: cancelling ? 4 : 3, source: source)
        compositionSource = nil
        queriedSource = cancelling ? nil : self.source()
    }
    func attributedSubstring(forProposedRange range: NSRange, actualRange: NSRangePointer?) -> NSAttributedString? {
        guard let source = query(), range.location != NSNotFound, range.location >= 0, range.length >= 0,
              range.location <= source.length else { return nil }
        let length = min(range.length, source.length - range.location)
        let valid = NSRange(location: range.location, length: length)
        // UIKit/AppKit may query adjacent text; never split UTF-16 surrogates.
        guard source.isScalarBoundary(valid.location), source.isScalarBoundary(valid.location + valid.length),
              let slice = Range(valid, in: source.text) else { return nil }
        actualRange?.pointee = valid
        return NSAttributedString(string: String(source.text[slice]))
    }
    func characterIndex(for point: NSPoint) -> Int {
        guard let source = query(), let view, let window = view.window else { return NSNotFound }
        let local = view.convert(window.convertPoint(fromScreen: point), from: nil)
        let logical = CGPoint(x: local.x, y: view.isFlipped ? local.y : view.bounds.height - local.y)
        return host?.queryNativeText(source: source, range: nil, point: logical)?.position ?? NSNotFound
    }
    func firstRect(forCharacterRange range: NSRange, actualRange: NSRangePointer?) -> NSRect {
        guard let source = query(), let view, let window = view.window else { return .zero }
        let requested = range.location == NSNotFound ? source.selectedRange : range
        guard let answer = host?.queryNativeText(source: source, range: requested, point: nil) else { return .zero }
        var rect = answer.caret.flatMap(NativeTextAnswer.rect) ?? .zero
        if let selections = answer.selection?.compactMap(NativeTextAnswer.rect), let first = selections.first {
            rect = first
            for next in selections.dropFirst() where abs(next.minY - first.minY) < 0.01 { rect = rect.union(next) }
        }
        if let firstRange = answer.firstRange, firstRange.count == 2 {
            actualRange?.pointee = NSRange(location: firstRange[0], length: firstRange[1] - firstRange[0])
        } else { actualRange?.pointee = requested }
        if !view.isFlipped { rect.origin.y = view.bounds.height - rect.maxY }
        return window.convertToScreen(view.convert(rect, to: nil))
    }
    func doCommand(by selector: Selector) {
        guard let source = source() else { return }
        if source.marked != nil {
            if NSStringFromSelector(selector) == "cancelOperation:" {
                _ = host?.editText(kind: 4, source: source); compositionSource = nil; return
            }
            _ = host?.editText(kind: 3, source: source)
            compositionSource = nil
        }
        command(selector)
        queriedSource = self.source()
    }
}
#endif
