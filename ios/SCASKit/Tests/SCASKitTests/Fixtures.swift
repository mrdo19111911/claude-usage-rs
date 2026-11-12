// Shared test data: a feed v1 as docs/specs/mobile-feed.md describes it, and a fixed clock.
import Foundation
@testable import SCASKit

enum Fixtures {
    /// Thu 2025-10-09 08:53:20 UTC.
    static let now: Int64 = 1_760_000_000_000
    static let utc = TimeZone(identifier: "UTC")!
    static let hour: Int64 = 3_600_000
    static let minute: Int64 = 60_000

    /// a1: Claude Code's account, week not started (resetsAt null), a Fable scoped limit.
    /// a2: usage null. a3: Claude Desktop's account, error text, a field phones do not know.
    static let feedJSON = """
    { "v": 1, "version": "1.6.1", "now": 1760000000000, "lastPollAt": 1759999990000,
      "polling": false, "autoSwitch": true, "threshold": 95, "weekThreshold": 99,
      "accounts": [
        { "id": "a1", "email": "one@x.com", "plan": "Max 20x", "enabled": true,
          "cli": true, "desktop": false, "error": null,
          "usage": { "five": {"u": 9, "resetsAt": 1760016740000},
                     "week": {"u": 24, "resetsAt": null},
                     "fetchedAt": 1759999990000,
                     "scoped": [ {"name": "Fable", "u": 3, "resetsAt": 1760172800000} ] } },
        { "id": "a2", "email": "two@x.com", "plan": "Pro", "enabled": true,
          "cli": false, "desktop": false, "error": null, "usage": null },
        { "id": "a3", "email": "three@x.com", "plan": "Max 5x", "enabled": true,
          "cli": false, "desktop": true, "error": "token expired", "newThing": [1, 2],
          "usage": { "five": {"u": 100, "resetsAt": 1760003600000},
                     "week": {"u": 50, "resetsAt": 1760500000000}, "fetchedAt": 1759999000000 } }
      ] }
    """

    static func feed() throws -> Feed {
        try Feed.decode(Data(feedJSON.utf8))
    }

    static func account(_ id: String, five: Int? = nil, week: Int = 0,
                        fiveReset: Int64? = nil, weekReset: Int64? = nil,
                        cli: Bool = false, desktop: Bool = false) -> Account {
        let usage = five.map {
            Usage(five: Window(u: $0, resetsAt: fiveReset), week: Window(u: week, resetsAt: weekReset),
                  fetchedAt: nil, scoped: nil)
        }
        return Account(id: id, email: "\(id)@x.com", plan: nil, enabled: true,
                       cli: cli, desktop: desktop, error: nil, usage: usage)
    }
}
