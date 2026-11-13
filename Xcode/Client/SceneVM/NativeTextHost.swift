import Foundation
import CoreGraphics

struct NativeTextInput: Decodable, Equatable {
    let target: UInt64
    let revision: UInt64?
    let selection: [Int]?
    let area: [Float]
}
struct NativeTextSnapshot: Decodable {
    let target: UInt64
    let revision: UInt64
    let text: String
    var selection: [Int]
    let marked: [Int]?
    var length: Int { (text as NSString).length }
    func isScalarBoundary(_ position: Int) -> Bool {
        let units = text.utf16
        guard position >= 0, position <= units.count else { return false }
        if position == 0 || position == units.count { return true }
        let index = units.index(units.startIndex, offsetBy: position)
        let previous = units[units.index(before: index)]
        return !(0xDC00...0xDFFF).contains(units[index]) || !(0xD800...0xDBFF).contains(previous)
    }
    var selectedRange: NSRange {
        guard selection.count == 2 else { return NSRange(location: 0, length: 0) }
        return NSRange(location: min(selection[0], selection[1]), length: abs(selection[1] - selection[0]))
    }
    var markedRange: NSRange {
        guard let marked, marked.count == 2 else { return NSRange(location: NSNotFound, length: 0) }
        return NSRange(location: marked[0], length: marked[1] - marked[0])
    }
}
struct NativeTextAnswer: Decodable {
    let caret: [Float]?
    let selection: [[Float]]?
    let firstRange: [Int]?
    let position: Int?
    nonisolated static func rect(_ a: [Float]) -> CGRect? {
        guard a.count == 4 else { return nil }
        return CGRect(x: CGFloat(a[0]), y: CGFloat(a[1]), width: CGFloat(a[2]), height: CGFloat(a[3]))
    }
}

/// Native input clients own a fixed input target and only forward source/range
/// operations. Editing, undo and glyph rendering remain in the Rust controller.
@MainActor
protocol NativeTextHost: AnyObject {
    var nativeInputState: NativeTextInput? { get }
    func nativeTextSnapshot() -> NativeTextSnapshot?
    func queryNativeText(source: NativeTextSnapshot, range: NSRange?, point: CGPoint?) -> NativeTextAnswer?
    func navigateNativeText(source: NativeTextSnapshot, position: Int, direction: Int, count: Int) -> Int?
    @discardableResult
    func editNativeText(kind: UInt32, source: NativeTextSnapshot, range: NSRange?, text: String, selection: NSRange?) -> Bool
}
extension NativeTextHost {
    @discardableResult
    func editText(kind: UInt32, source: NativeTextSnapshot, range: NSRange? = nil, text: String = "", selection: NSRange? = nil) -> Bool {
        editNativeText(kind: kind, source: source, range: range, text: text, selection: selection)
    }
}

