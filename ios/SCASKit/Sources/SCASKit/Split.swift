// The accounts screen's three groups, as the tray widget's split_others (src/tray/model.rs):
// - inUse: Claude Code's (cli) or Claude Desktop's (desktop) account, feed order; one row each.
// - switchTo: the rest that are not full, most room first (key: max(five, week), then week,
//   then email); no figures last.
// - usedUp: full (five >= 100 or week >= 100), soonest free first; unknown time last.
import Foundation

public struct UsedUp: Equatable, Sendable {
    public var account: Account
    /// When it can be used again: the latest reset among its full windows; nil: not known.
    public var freeAt: Int64?
}

public struct Split: Equatable, Sendable {
    public var inUse: [Account]
    public var switchTo: [Account]
    public var usedUp: [UsedUp]

    /// The account the widget shows first: Claude Code's, else Claude Desktop's.
    public var main: Account? { inUse.first(where: \.cli) ?? inUse.first }
    /// The best account to switch to.
    public var best: Account? { switchTo.first }

    public static func of(_ accounts: [Account]) -> Split {
        var inUse: [Account] = [], open: [Account] = [], full: [UsedUp] = []
        for a in accounts {
            if a.cli || a.desktop {
                inUse.append(a)
            } else if let u = a.usage, let at = freeAt(u) {
                full.append(UsedUp(account: a, freeAt: at))
            } else {
                open.append(a)
            }
        }
        open.sort(by: roomFirst)
        full.sort { x, y in
            let (a, b) = (x.freeAt ?? .max, y.freeAt ?? .max)
            return a != b ? a < b : x.account.email < y.account.email
        }
        return Split(inUse: inUse, switchTo: open, usedUp: full)
    }

    /// nil: not full. .some(nil): full, time not known. .some(t): free again at t.
    public static func freeAt(_ u: Usage) -> Int64?? {
        let full = [u.five, u.week].filter { $0.u >= 100 }
        if full.isEmpty { return nil }
        if full.contains(where: { $0.resetsAt == nil }) { return .some(nil) }
        return .some(full.compactMap(\.resetsAt).max())
    }

    static func roomFirst(_ a: Account, _ b: Account) -> Bool {
        switch (key(a), key(b)) {
        case (nil, nil): return a.email < b.email
        case (nil, _): return false
        case (_, nil): return true
        case let (ka?, kb?):
            if ka != kb { return ka.0 != kb.0 ? ka.0 < kb.0 : ka.1 < kb.1 }
            return a.email < b.email
        }
    }

    static func key(_ a: Account) -> (Int, Int)? {
        a.usage.map { (max($0.five.u, $0.week.u), $0.week.u) }
    }
}
