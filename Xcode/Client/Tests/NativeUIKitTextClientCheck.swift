#if os(iOS)
import UIKit

@MainActor
private final class RecordedUIKitHost: NativeTextHost {
    var snapshot = NativeTextSnapshot(target: 41, revision: 1, text: "A👩Z", selection: [3,3], marked: nil)
    var next: NativeTextSnapshot?
    var edits: [(UInt32, UInt64, NSRange?, String)] = []
    var nativeInputState: NativeTextInput? { NativeTextInput(target:snapshot.target,revision:snapshot.revision,selection:snapshot.selection,area:[8,12,2,24]) }
    func nativeTextSnapshot() -> NativeTextSnapshot? { snapshot }
    func editNativeText(kind: UInt32, source: NativeTextSnapshot, range: NSRange?, text: String, selection: NSRange?) -> Bool {
        edits.append((kind,source.revision,range,text))
        if let next { snapshot=next; self.next=nil }
        return true
    }
    func queryNativeText(source: NativeTextSnapshot, range: NSRange?, point: CGPoint?) -> NativeTextAnswer? {
        NativeTextAnswer(caret:[8,12,1,24],selection:[[8,12,20,24]],firstRange:[1,3],position:3)
    }
    func navigateNativeText(source: NativeTextSnapshot, position: Int, direction: Int, count: Int) -> Int? { direction == 0 ? 1 : 3 }
}
/// Ready for an iOS runtime harness; SDK typechecking alone does not execute it.
@MainActor
func checkNativeUIKitTextClient() {
    let host=RecordedUIKitHost()
    let view=UIView(frame:CGRect(x:0,y:0,width:320,height:120))
    let client=NativeUIKitTextClient(target:41,host:host,view:view)
    let first=client.beginningOfDocument
    let end=client.endOfDocument
    let whole=client.textRange(from:first,to:end)!
    precondition(client.text(in:whole)=="A👩Z")
    precondition(client.position(from:first,offset:2)==nil,"Reject a surrogate split")
    let emojiStart=client.position(from:first,offset:1)!
    let emojiEnd=client.position(from:first,offset:3)!
    let emoji=client.textRange(from:emojiStart,to:emojiEnd)!
    precondition(client.text(in:emoji)=="👩")
    precondition(client.firstRect(for:emoji)==CGRect(x:8,y:12,width:20,height:24))
    precondition(client.selectionRects(for:emoji).count==1)
    precondition(client.offset(from:first,to:client.closestPosition(to:.zero)!)==3)
    host.next=NativeTextSnapshot(target:41,revision:2,text:"A日本Z",selection:[3,3],marked:[1,3])
    client.setMarkedText("日本",selectedRange:NSRange(location:2,length:0))
    precondition(host.edits.last!.0==2 && host.edits.last!.1==1)
    precondition(client.markedTextRange != nil)
    let count=host.edits.count
    client.replace(whole,withText:"stale source")
    precondition(host.edits.count==count)
    host.next=NativeTextSnapshot(target:41,revision:3,text:"A日Z",selection:[2,2],marked:[1,2])
    client.deleteBackward()
    precondition(host.edits.last!.0==6 && host.edits.last!.1==2)
    host.next=NativeTextSnapshot(target:41,revision:4,text:"A日Z",selection:[2,2],marked:nil)
    client.unmarkText()
    precondition(host.edits.last!.0==3 && client.markedTextRange==nil)
    let retiredCount=host.edits.count
    host.snapshot=NativeTextSnapshot(target:99,revision:9,text:"Other field",selection:[0,0],marked:nil)
    client.insertText("late"); client.deleteBackward(); client.replace(whole,withText:"late")
    precondition(host.edits.count==retiredCount && client.selectedTextRange==nil)
    client.deactivate()
    precondition(!client.canBecomeFirstResponder)
}
#endif
