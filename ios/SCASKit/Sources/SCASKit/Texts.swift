// Every word and number the user reads, as pure functions of (value, now, time zone).
// Mirrors the tray widget (src/tray/words.rs, model.rs `left`/`clock`; docs/specs/modules.md
// "Widget v2" and "Not started yet"). Times are ms since 1970.
import Foundation

public enum Texts {
    public static let inUse = "In use now"
    public static let switchTo = "Switch to"
    public static let usedUp = "Used up · available at"
    public static let notChecked = "Usage not checked yet"
    public static let notStarted = "not started yet"
    public static let notKnown = "not known"
    public static let figuresKey = "5-hour · weekly"
    public static let unreachable = "PC not reachable (same Wi-Fi or Tailscale needed)"

    /// "4h39m", "2d 3h", "12m"; rounds up to the minute, never below "0m".
    public static func left(_ ms: Int64) -> String {
        let m = (max(ms, 0) + 59_999) / 60_000
        if m >= 1440 { return "\(m / 1440)d \(m % 1440 / 60)h" }
        if m >= 60 { return "\(m / 60)h\(two(Int(m % 60)))m" }
        return "\(m)m"
    }

    /// "22:20" when `t` is on the same local day as `now`, else "Thu 11:00".
    public static func clock(_ t: Int64, now: Int64, tz: TimeZone) -> String {
        var cal = Calendar(identifier: .gregorian)
        cal.timeZone = tz
        let a = cal.dateComponents([.year, .month, .day, .hour, .minute, .weekday], from: date(t))
        let b = cal.dateComponents([.year, .month, .day], from: date(now))
        let hm = "\(two(a.hour ?? 0)):\(two(a.minute ?? 0))"
        if (a.year, a.month, a.day) == (b.year, b.month, b.day) { return hm }
        let days = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"]
        return "\(days[((a.weekday ?? 1) - 1) % 7]) \(hm)"
    }

    /// "resets in 4h39m · 22:20"; "not started yet" when the window has no reset.
    public static func resets(_ t: Int64?, now: Int64, tz: TimeZone) -> String {
        guard let t else { return notStarted }
        return "resets in \(left(t - now)) · \(clock(t, now: now, tz: tz))"
    }

    public static func percent(_ u: Int) -> String { "\(u)%" }

    /// Bar length, 0...1.
    public static func fraction(_ u: Int) -> Double { min(max(Double(u) / 100, 0), 1) }

    /// "26% · 74%" (5-hour, weekly); "—" without figures.
    public static func figures(_ u: Usage?) -> String {
        guard let u else { return "—" }
        return "\(percent(u.five.u)) · \(percent(u.week.u))"
    }

    /// When a used-up account can be used again; "not known" without a time.
    public static func availableAt(_ t: Int64?, now: Int64, tz: TimeZone) -> String {
        guard let t else { return notKnown }
        return clock(t, now: now, tz: tz)
    }

    /// A per-model weekly limit: "Fable 3%".
    public static func chip(_ s: Scoped) -> String { "\(s.name) \(percent(s.u))" }

    /// Under stale figures: "Last seen 14:02 · PC not reachable (...)".
    public static func lastSeen(_ seenAt: Int64?, now: Int64, tz: TimeZone) -> String {
        guard let seenAt else { return unreachable }
        return "Last seen \(clock(seenAt, now: now, tz: tz)) · \(unreachable)"
    }

    /// One line for a FeedError.
    public static func problem(_ e: FeedError) -> String {
        switch e {
        case .notPaired: return "Not paired with a PC yet."
        case .unauthorized: return "The PC refused the pairing. Pair again."
        case .forbidden: return "The PC does not accept this network. Use the same Wi-Fi or Tailscale."
        case .unreachable: return unreachable
        case .badStatus(let code): return "The PC answered \(code)."
        case .badData: return "The PC's answer could not be read."
        case .unsupportedVersion(let v): return "The PC sends feed v\(v); update this app."
        }
    }

    /// Email without the domain: "one" for "one@x.com".
    public static func shortName(_ email: String) -> String {
        email.split(separator: "@").first.map(String.init) ?? email
    }

    /// The phone's clock in ms since 1970.
    public static func nowMs(_ d: Date = Date()) -> Int64 { Int64(d.timeIntervalSince1970 * 1000) }

    /// "05" for 5.
    static func two(_ n: Int) -> String { n < 10 ? "0\(n)" : "\(n)" }

    static func date(_ ms: Int64) -> Date { Date(timeIntervalSince1970: Double(ms) / 1000) }
}
