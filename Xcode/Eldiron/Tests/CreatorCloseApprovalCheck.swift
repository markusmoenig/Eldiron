@main
struct CreatorCloseApprovalCheck {
    static func main() {
        var first = CreatorCloseApproval()
        var second = CreatorCloseApproval()
        precondition(!first.requiresConfirmation(host: 1, hasChanges: false))
        precondition(first.requiresConfirmation(host: 1, hasChanges: true))
        first.approve(host: 1)
        precondition(!first.requiresConfirmation(host: 1, hasChanges: true))
        precondition(second.requiresConfirmation(host: 2, hasChanges: true),
                     "Closing one dirty window must not approve another window's Quit")
        precondition(first.requiresConfirmation(host: 3, hasChanges: true),
                     "A recreated host cannot inherit a stale discard decision")
        first.reset()
        precondition(first.requiresConfirmation(host: 1, hasChanges: true))
        second.approve(host: 0)
        precondition(second.requiresConfirmation(host: 0, hasChanges: true))
        print("Creator close approvals remain scoped to their closing owner")
    }
}
