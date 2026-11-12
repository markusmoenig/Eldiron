#if os(iOS)
import UIKit

/// Native positions address one immutable shared-editor source. A delayed range
/// from another field or revision is never reinterpreted in the current field.
final class NativeTextPosition: UITextPosition {
    let target: UInt64
    let revision: UInt64
    let offset: Int
    init(_ source: NativeTextSnapshot, _ offset: Int) {
        target = source.target; revision = source.revision; self.offset = offset
        super.init()
    }
}
final class NativeTextRange: UITextRange {
    let lower: NativeTextPosition
    let upper: NativeTextPosition
    init(_ source: NativeTextSnapshot, _ range: NSRange) {
        lower = NativeTextPosition(source, range.location)
        upper = NativeTextPosition(source, range.location + range.length)
        super.init()
    }
    override var start: UITextPosition { lower }
    override var end: UITextPosition { upper }
    override var isEmpty: Bool { lower.offset == upper.offset }
}
private final class NativeSelectionRect: UITextSelectionRect {
    private let bounds: CGRect
    private let first: Bool
    private let last: Bool
    init(_ rect: CGRect, first: Bool, last: Bool) {
        bounds = rect; self.first = first; self.last = last; super.init()
    }
    override var rect: CGRect { bounds }
    override var writingDirection: NSWritingDirection { .natural }
    override var containsStart: Bool { first }
    override var containsEnd: Bool { last }
    override var isVertical: Bool { false }
}

/// A zero-size, non-drawing responder, replaced whenever logical focus changes.
/// Rust remains the sole owner of text, marked state, undo and painted geometry.
final class NativeUIKitTextClient: UIView, UITextInput {
    let target: UInt64
    private weak var host: (any NativeTextHost)?
    private weak var coordinateView: UIView?
    private var compositionSource: NativeTextSnapshot?
    private var observed: NativeTextSnapshot?
    private var editing = false
    private var notifying = false
    private var retired = false
    weak var inputDelegate: (any UITextInputDelegate)?
    var markedTextStyle: [NSAttributedString.Key: Any]?
    lazy var tokenizer: any UITextInputTokenizer = UITextInputStringTokenizer(textInput: self)
    override var canBecomeFirstResponder: Bool { !retired }
    var textInputView: UIView { coordinateView ?? self }
    var autocorrectionType: UITextAutocorrectionType = .no
    var spellCheckingType: UITextSpellCheckingType = .no
    var smartQuotesType: UITextSmartQuotesType = .no
    var smartDashesType: UITextSmartDashesType = .no
    init(target: UInt64, host: any NativeTextHost, view: UIView) {
        self.target = target; self.host = host; coordinateView = view
        super.init(frame: .zero)
        backgroundColor = .clear; isUserInteractionEnabled = false
        observed = source()
    }
    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }
    private func source() -> NativeTextSnapshot? {
        if notifying { return observed }
        guard !retired, let source = host?.nativeTextSnapshot(), source.target == target else { return nil }
        return source
    }
    private func position(_ value: UITextPosition, _ source: NativeTextSnapshot) -> Int? {
        guard let value = value as? NativeTextPosition, value.target == target,
              value.revision == source.revision, source.isScalarBoundary(value.offset) else { return nil }
        return value.offset
    }
    private func range(_ value: UITextRange, _ source: NativeTextSnapshot) -> NSRange? {
        guard let lo = position(value.start, source), let hi = position(value.end, source), lo <= hi else { return nil }
        return NSRange(location: lo, length: hi - lo)
    }
    private func geometry(_ source: NativeTextSnapshot, _ range: NSRange) -> NativeTextAnswer? {
        host?.queryNativeText(source: source, range: range, point: nil)
    }
    private func edit(_ kind: UInt32, source: NativeTextSnapshot, range: NSRange? = nil, text: String = "", selection: NSRange? = nil) {
        guard !retired, !editing, !notifying else { return }
        editing = true
        _ = host?.editText(kind: kind, source: source, range: range, text: text, selection: selection)
        observed = self.source()
        editing = false
    }
    /// Notify UIKit when touch/keyboard/clipboard/runtime changes come through
    /// the common host, excluding edits already initiated by UIKit itself.
    func synchronize() {
        guard !editing, !retired, !notifying else { return }
        let next = source()
        let textChanged = observed?.revision != next?.revision
        let selectionChanged = textChanged || observed?.selection != next?.selection
        notifying = true
        if textChanged { inputDelegate?.textWillChange(self) }
        if selectionChanged { inputDelegate?.selectionWillChange(self) }
        observed = next
        if selectionChanged { inputDelegate?.selectionDidChange(self) }
        if textChanged { inputDelegate?.textDidChange(self) }
        notifying = false
    }
    func deactivate() {
        guard !retired else { return }
        if let source = compositionSource ?? source() { edit(4, source: source) }
        retired = true; compositionSource = nil; observed = nil
        resignFirstResponder(); inputDelegate = nil
    }
    var hasText: Bool { source()?.text.isEmpty == false }
    func insertText(_ text: String) {
        guard source() != nil, let source = compositionSource ?? source() else { return }
        edit(5, source: source, text: text); compositionSource = nil
    }
    func deleteBackward() {
        guard source() != nil, let source = compositionSource ?? source() else { return }
        // Shared controller deletes a grapheme, including within marked text.
        edit(6, source: source)
        compositionSource = self.source()?.marked == nil ? nil : self.source()
    }
    func text(in value: UITextRange) -> String? {
        guard let source = source(), let r = range(value, source), let slice = Range(r, in: source.text) else { return nil }
        return String(source.text[slice])
    }
    func replace(_ value: UITextRange, withText text: String) {
        guard let source = source(), let r = range(value, source) else { return }
        edit(1, source: source, range: r, text: text); compositionSource = nil
    }
    var selectedTextRange: UITextRange? {
        get { source().map { NativeTextRange($0, $0.selectedRange) } }
        set {
            guard let source = source(), let value = newValue, let r = range(value, source) else { return }
            edit(0, source: source, range: r)
            compositionSource = self.source()?.marked == nil ? nil : self.source()
        }
    }
    var markedTextRange: UITextRange? {
        guard let source = source(), source.marked != nil else { return nil }
        return NativeTextRange(source, source.markedRange)
    }
    func setMarkedText(_ markedText: String?, selectedRange: NSRange) {
        guard source() != nil, let source = compositionSource ?? source() else { return }
        edit(2, source: source, text: markedText ?? "", selection: selectedRange)
        compositionSource = self.source()?.marked == nil ? nil : self.source()
    }
    func unmarkText() {
        guard let source = compositionSource ?? source() else { return }
        edit(3, source: source); compositionSource = nil
    }
    // UIKit requires nonoptional endpoints even while a responder is retiring.
    // Such sentinel positions cannot pass the source/target checks above.
    private var emptySource: NativeTextSnapshot { NativeTextSnapshot(target: 0, revision: 0, text: "", selection: [0,0], marked: nil) }
    var beginningOfDocument: UITextPosition { NativeTextPosition(source() ?? emptySource, 0) }
    var endOfDocument: UITextPosition { let s = source() ?? emptySource; return NativeTextPosition(s, s.length) }
    func textRange(from: UITextPosition, to: UITextPosition) -> UITextRange? {
        guard let s = source(), let a = position(from, s), let b = position(to, s) else { return nil }
        return NativeTextRange(s, NSRange(location: min(a,b), length: abs(b-a)))
    }
    func position(from value: UITextPosition, offset: Int) -> UITextPosition? {
        guard let s = source(), let p = position(value, s) else { return nil }
        let sum = p.addingReportingOverflow(offset)
        guard !sum.overflow, s.isScalarBoundary(sum.partialValue) else { return nil }
        return NativeTextPosition(s, sum.partialValue)
    }
    private func direction(_ value: UITextLayoutDirection) -> Int? {
        switch value { case .left: return 0; case .right: return 1; case .up: return 2; case .down: return 3; @unknown default: return nil }
    }
    func position(from value: UITextPosition, in layoutDirection: UITextLayoutDirection, offset: Int) -> UITextPosition? {
        guard let s = source(), let p = position(value, s), var d = direction(layoutDirection), offset != Int.min else { return nil }
        if offset < 0 { d ^= 1 }
        guard let result = host?.navigateNativeText(source: s, position: p, direction: d, count: abs(offset)) else { return nil }
        return NativeTextPosition(s, result)
    }
    func compare(_ a: UITextPosition, to b: UITextPosition) -> ComparisonResult {
        guard let s = source(), let a = position(a,s), let b = position(b,s) else { return .orderedSame }
        return a == b ? .orderedSame : (a < b ? .orderedAscending : .orderedDescending)
    }
    func offset(from a: UITextPosition, to b: UITextPosition) -> Int {
        guard let s = source(), let a = position(a,s), let b = position(b,s) else { return 0 }
        return b-a
    }
    func position(within value: UITextRange, farthestIn layoutDirection: UITextLayoutDirection) -> UITextPosition? {
        guard let s = source(), let r = range(value,s), let d = direction(layoutDirection) else { return nil }
        // Resolve the painted selection extremum and hit it through the shared
        // layout. This avoids one FFI/layout query per character in long text.
        guard let a = geometry(s,r) else { return nil }
        let rects = a.selection?.compactMap(NativeTextAnswer.rect) ?? []
        let chosen: CGRect?
        switch d {
        case 0: chosen = rects.min { $0.minX < $1.minX }
        case 1: chosen = rects.max { $0.maxX < $1.maxX }
        case 2: chosen = rects.min { $0.minY < $1.minY }
        default: chosen = rects.max { $0.maxY < $1.maxY }
        }
        guard let rect = chosen else { return NativeTextPosition(s,r.location) }
        let point = CGPoint(x: d == 1 || d == 3 ? rect.maxX : rect.minX,y:rect.midY)
        guard let hit = host?.queryNativeText(source:s,range:nil,point:point)?.position else { return nil }
        return NativeTextPosition(s,min(r.location+r.length,max(r.location,hit)))
    }

    func characterRange(byExtending value: UITextPosition, in layoutDirection: UITextLayoutDirection) -> UITextRange? {
        guard let s = source(), let p = position(value,s), let d = direction(layoutDirection),
              let next = host?.navigateNativeText(source:s,position:p,direction:d,count:1), next != p else { return nil }
        return NativeTextRange(s,NSRange(location:min(p,next),length:abs(next-p)))
    }
    func baseWritingDirection(for position: UITextPosition, in direction: UITextStorageDirection) -> NSWritingDirection { .natural }
    func setBaseWritingDirection(_ writingDirection: NSWritingDirection, for range: UITextRange) {
        // Shared plain-text editors derive bidi direction from their contents;
        // there is no serialized per-range rich-text paragraph attribute.
    }
    func firstRect(for value: UITextRange) -> CGRect {
        guard let s = source(), let r = range(value,s), let a = geometry(s,r) else { return .zero }
        let rects = a.selection?.compactMap(NativeTextAnswer.rect) ?? []
        guard let first = rects.first else { return a.caret.flatMap(NativeTextAnswer.rect) ?? .zero }
        return rects.dropFirst().filter { abs($0.minY-first.minY) < 0.01 }.reduce(first) { $0.union($1) }
    }
    func caretRect(for value: UITextPosition) -> CGRect {
        guard let s = source(), let p = position(value,s) else { return .zero }
        return geometry(s,NSRange(location:p,length:0))?.caret.flatMap(NativeTextAnswer.rect) ?? .zero
    }
    func selectionRects(for value: UITextRange) -> [UITextSelectionRect] {
        guard let s = source(), let r = range(value,s), let a = geometry(s,r) else { return [] }
        let rects = a.selection?.compactMap(NativeTextAnswer.rect) ?? []
        let start = geometry(s,NSRange(location:r.location,length:0))?.caret.flatMap(NativeTextAnswer.rect)
        let end = geometry(s,NSRange(location:r.location+r.length,length:0))?.caret.flatMap(NativeTextAnswer.rect)
        return rects.map { rect in
            func contains(_ caret: CGRect?) -> Bool {
                guard let caret else { return false }
                return rect.insetBy(dx:-1,dy:-1).contains(CGPoint(x:caret.minX,y:caret.midY))
            }
            return NativeSelectionRect(rect,first:contains(start),last:contains(end))
        }
    }
    func closestPosition(to point: CGPoint) -> UITextPosition? {
        guard let s = source(), let p = host?.queryNativeText(source:s,range:nil,point:point)?.position else { return nil }
        return NativeTextPosition(s,p)
    }
    func closestPosition(to point: CGPoint, within value: UITextRange) -> UITextPosition? {
        guard let s = source(), let r = range(value,s), let hit = closestPosition(to:point), let p = position(hit,s) else { return nil }
        return NativeTextPosition(s,min(r.location+r.length,max(r.location,p)))
    }
    func characterRange(at point: CGPoint) -> UITextRange? {
        guard let s = source(), let hit = closestPosition(to:point), let p = position(hit,s), s.length > 0 else { return nil }
        let r = (s.text as NSString).rangeOfComposedCharacterSequence(at:min(p,s.length-1))
        return NativeTextRange(s,r)
    }
}
#endif
