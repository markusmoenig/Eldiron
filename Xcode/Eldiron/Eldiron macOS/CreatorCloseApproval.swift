// A discard decision belongs to one closing Creator, never to another window.
struct CreatorCloseApproval {
    private var approvedHost: UInt64?

    mutating func approve(host: UInt64) {
        approvedHost = host == 0 ? nil : host
    }
    mutating func reset() { approvedHost = nil }
    func requiresConfirmation(host: UInt64, hasChanges: Bool) -> Bool {
        hasChanges && (host == 0 || approvedHost != host)
    }
}
