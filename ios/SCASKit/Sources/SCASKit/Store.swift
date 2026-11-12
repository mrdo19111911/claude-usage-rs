// What the app and the widget share: host and port and the last snapshot in the App Group's
// defaults (`group.com.smartlog.scas`, .standard when the group is not available), the token
// in a TokenVault (the Keychain in the app). "Forget this PC" removes all three.
import Foundation

public protocol TokenVault {
    func read() -> String?
    @discardableResult func write(_ token: String) -> Bool
    func delete()
}

/// The last figures that came in, and when (phone clock, ms).
public struct Snapshot: Equatable, Sendable {
    public var feed: Feed
    public var seenAt: Int64
}

public final class Store {
    public static let appGroup = "group.com.smartlog.scas"
    enum Key {
        static let host = "pair.host", port = "pair.port"
        static let feed = "snapshot.feed", seenAt = "snapshot.seenAt"
    }

    let defaults: UserDefaults
    let vault: TokenVault

    public init(defaults: UserDefaults, vault: TokenVault) {
        self.defaults = defaults; self.vault = vault
    }

    /// The App Group's store with the Keychain, as the app and the widget use it.
    public static func shared() -> Store {
        Store(defaults: UserDefaults(suiteName: appGroup) ?? .standard, vault: KeychainVault())
    }

    public func pairing() -> Pairing? {
        guard let host = defaults.string(forKey: Key.host), !host.isEmpty,
              defaults.object(forKey: Key.port) != nil,
              let token = vault.read(), !token.isEmpty else { return nil }
        return Pairing(host: host, port: defaults.integer(forKey: Key.port), token: token)
    }

    public func save(_ p: Pairing) {
        defaults.set(p.host, forKey: Key.host)
        defaults.set(p.port, forKey: Key.port)
        vault.write(p.token)
    }

    public func forget() {
        for k in [Key.host, Key.port, Key.feed, Key.seenAt] { defaults.removeObject(forKey: k) }
        vault.delete()
    }

    public func saveSnapshot(_ body: Data, seenAt: Int64) {
        defaults.set(body, forKey: Key.feed)
        defaults.set(seenAt, forKey: Key.seenAt)
    }

    public func snapshot() -> Snapshot? {
        guard let body = defaults.data(forKey: Key.feed), let feed = try? Feed.decode(body) else { return nil }
        let seen = (defaults.object(forKey: Key.seenAt) as? NSNumber)?.int64Value ?? 0
        return Snapshot(feed: feed, seenAt: seen)
    }
}

/// What a screen or the widget draws: fresh figures, or the last ones with the reason.
public struct Loaded: Equatable, Sendable {
    public var feed: Feed?
    public var seenAt: Int64?
    public var error: FeedError?
    /// Figures shown are not fresh (draw them greyed).
    public var stale: Bool { error != nil }

    public init(feed: Feed?, seenAt: Int64?, error: FeedError?) {
        self.feed = feed; self.seenAt = seenAt; self.error = error
    }
}

public enum Loader {
    /// Fetch; on success keep the body as the snapshot; on failure fall back to the snapshot.
    public static func load(store: Store, source: FeedSource, now: Int64) async -> Loaded {
        guard let p = store.pairing() else { return Loaded(feed: nil, seenAt: nil, error: .notPaired) }
        do {
            let (feed, body) = try await source.fetch(p)
            store.saveSnapshot(body, seenAt: now)
            return Loaded(feed: feed, seenAt: now, error: nil)
        } catch {
            let e = FeedClient.map(error)
            let last = store.snapshot()
            return Loaded(feed: last?.feed, seenAt: last?.seenAt, error: e)
        }
    }

    /// Only the stored figures (the widget's placeholder, the app's first frame).
    public static func stored(_ store: Store) -> Loaded {
        let last = store.snapshot()
        return Loaded(feed: last?.feed, seenAt: last?.seenAt, error: last == nil ? .notPaired : nil)
    }
}
