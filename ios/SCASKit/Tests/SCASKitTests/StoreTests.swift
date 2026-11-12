// Pairing (host/port in defaults, token in the vault) and the last snapshot; the loader.
import XCTest
@testable import SCASKit

final class MemoryVault: TokenVault {
    var token: String?
    func read() -> String? { token }
    func write(_ token: String) -> Bool { self.token = token; return true }
    func delete() { token = nil }
}

struct StubSource: FeedSource {
    var result: Result<Data, FeedError>
    func fetch(_ pairing: Pairing) async throws -> (Feed, Data) {
        let data = try result.get()
        return (try Feed.decode(data), data)
    }
}

final class StoreTests: XCTestCase {
    var defaults: UserDefaults!
    var vault: MemoryVault!
    var store: Store!
    let pairing = Pairing(host: "10.0.0.2", port: 4778, token: "tok")

    override func setUp() {
        let suite = "scas-test-\(UUID().uuidString)"
        defaults = UserDefaults(suiteName: suite)!
        vault = MemoryVault()
        store = Store(defaults: defaults, vault: vault)
    }

    func testPairingRoundTripAndTokenOnlyInVault() {
        XCTAssertNil(store.pairing())
        store.save(pairing)
        XCTAssertEqual(store.pairing(), pairing)
        XCTAssertEqual(vault.token, "tok")
        let all = defaults.dictionaryRepresentation().values.compactMap { $0 as? String }
        XCTAssertFalse(all.contains("tok"), "token never in defaults")
    }

    func testNoTokenMeansNotPaired() {
        store.save(pairing)
        vault.delete()
        XCTAssertNil(store.pairing())
    }

    func testForgetRemovesEverything() {
        store.save(pairing)
        store.saveSnapshot(Data(Fixtures.feedJSON.utf8), seenAt: 5)
        store.forget()
        XCTAssertNil(store.pairing())
        XCTAssertNil(store.snapshot())
        XCTAssertNil(vault.token)
    }

    func testSnapshotRoundTrip() throws {
        store.saveSnapshot(Data(Fixtures.feedJSON.utf8), seenAt: 42)
        let s = try XCTUnwrap(store.snapshot())
        XCTAssertEqual(s.seenAt, 42)
        XCTAssertEqual(s.feed.accounts.count, 3)
    }

    func testLoadNotPaired() async {
        let got = await Loader.load(store: store, source: StubSource(result: .failure(.forbidden)), now: 9)
        XCTAssertEqual(got.error, .notPaired)
        XCTAssertNil(got.feed)
    }

    func testLoadSuccessSavesSnapshot() async {
        store.save(pairing)
        let ok = StubSource(result: .success(Data(Fixtures.feedJSON.utf8)))
        let got = await Loader.load(store: store, source: ok, now: 77)
        XCTAssertNil(got.error)
        XCTAssertEqual(got.seenAt, 77)
        XCTAssertFalse(got.stale)
        XCTAssertEqual(store.snapshot()?.seenAt, 77)
    }

    func testLoadFailureFallsBackToSnapshot() async {
        store.save(pairing)
        store.saveSnapshot(Data(Fixtures.feedJSON.utf8), seenAt: 50)
        let bad = StubSource(result: .failure(.unreachable("offline")))
        let got = await Loader.load(store: store, source: bad, now: 99)
        XCTAssertEqual(got.feed?.accounts.count, 3)
        XCTAssertEqual(got.seenAt, 50)
        XCTAssertTrue(got.stale)
        XCTAssertEqual(got.error, .unreachable("offline"))
    }
}
