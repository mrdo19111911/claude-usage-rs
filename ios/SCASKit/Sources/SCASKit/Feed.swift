// Feed v1: what `GET /api/mobile/v1/state` answers (docs/specs/mobile-feed.md "The JSON").
// In: the response body. Out: `Feed`, or FeedError.unsupportedVersion / .badData.
// Times are ms since 1970 (Int64). Unknown fields are ignored; nulls stay nil.
import Foundation

public struct Feed: Codable, Equatable, Sendable {
    public var v: Int
    public var version: String?
    public var now: Int64
    public var lastPollAt: Int64?
    public var polling: Bool?
    public var autoSwitch: Bool?
    public var threshold: Int?
    public var weekThreshold: Int?
    public var accounts: [Account]

    /// The only feed version this app reads.
    public static let supported = 1

    public static func decode(_ data: Data) throws -> Feed {
        let feed: Feed
        do {
            feed = try JSONDecoder().decode(Feed.self, from: data)
        } catch {
            throw FeedError.badData(String(describing: error))
        }
        guard feed.v == supported else { throw FeedError.unsupportedVersion(feed.v) }
        return feed
    }
}

public struct Account: Codable, Equatable, Identifiable, Sendable {
    public var id: String
    public var email: String
    public var plan: String?
    public var enabled: Bool
    /// Claude Code uses this account now.
    public var cli: Bool
    /// Claude Desktop is signed in to this account now.
    public var desktop: Bool
    public var error: String?
    public var usage: Usage?

    public init(id: String, email: String, plan: String?, enabled: Bool, cli: Bool, desktop: Bool,
                error: String?, usage: Usage?) {
        self.id = id; self.email = email; self.plan = plan; self.enabled = enabled
        self.cli = cli; self.desktop = desktop; self.error = error; self.usage = usage
    }

    enum CodingKeys: String, CodingKey { case id, email, plan, enabled, cli, desktop, error, usage }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        id = try c.decode(String.self, forKey: .id)
        email = try c.decodeIfPresent(String.self, forKey: .email) ?? "?"
        plan = try? c.decodeIfPresent(String.self, forKey: .plan)
        enabled = (try? c.decodeIfPresent(Bool.self, forKey: .enabled)) ?? true
        cli = (try? c.decodeIfPresent(Bool.self, forKey: .cli)) ?? false
        desktop = (try? c.decodeIfPresent(Bool.self, forKey: .desktop)) ?? false
        // An error that is not text (a future shape) still reads as an error.
        if let text = try? c.decodeIfPresent(String.self, forKey: .error) {
            error = text
        } else {
            error = (try? c.decodeNil(forKey: .error)) == false ? "error" : nil
        }
        usage = try c.decodeIfPresent(Usage.self, forKey: .usage)
    }
}

public struct Usage: Codable, Equatable, Sendable {
    public var five: Window
    public var week: Window
    public var fetchedAt: Int64?
    /// Per-model weekly limits ("Fable"); nil when the PC sent none.
    public var scoped: [Scoped]?

    public init(five: Window, week: Window, fetchedAt: Int64?, scoped: [Scoped]?) {
        self.five = five; self.week = week; self.fetchedAt = fetchedAt; self.scoped = scoped
    }
}

/// One limit: percent used and its reset (nil: not started yet).
public struct Window: Codable, Equatable, Sendable {
    public var u: Int
    public var resetsAt: Int64?

    public init(u: Int, resetsAt: Int64?) { self.u = u; self.resetsAt = resetsAt }

    enum CodingKeys: String, CodingKey { case u, resetsAt }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        u = Window.percent(c, .u)
        resetsAt = try c.decodeIfPresent(Int64.self, forKey: .resetsAt)
    }

    /// A whole percent; a fractional one is rounded, a missing one is 0.
    static func percent<K: CodingKey>(_ c: KeyedDecodingContainer<K>, _ key: K) -> Int {
        if let i = try? c.decodeIfPresent(Int.self, forKey: key) { return i }
        if let d = try? c.decodeIfPresent(Double.self, forKey: key) { return Int(d.rounded()) }
        return 0
    }
}

public struct Scoped: Codable, Equatable, Sendable {
    public var name: String
    public var u: Int
    public var resetsAt: Int64?

    public init(name: String, u: Int, resetsAt: Int64?) {
        self.name = name; self.u = u; self.resetsAt = resetsAt
    }

    enum CodingKeys: String, CodingKey { case name, u, resetsAt }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        name = try c.decode(String.self, forKey: .name)
        u = Window.percent(c, .u)
        resetsAt = try c.decodeIfPresent(Int64.self, forKey: .resetsAt)
    }
}

/// Why there are no fresh figures.
public enum FeedError: Error, Equatable, Sendable {
    case notPaired
    case unauthorized          // 401: token wrong (PC re-paired or token file deleted)
    case forbidden             // 403: this network is not allowed by the PC
    case unreachable(String)   // no answer: other network, PC off, feed off
    case badStatus(Int)
    case badData(String)
    case unsupportedVersion(Int)
}
