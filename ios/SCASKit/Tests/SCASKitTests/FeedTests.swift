// Feed v1 decode (docs/specs/mobile-feed.md "The JSON").
import XCTest
@testable import SCASKit

final class FeedTests: XCTestCase {
    func testDecodesTheTopLevel() throws {
        let f = try Fixtures.feed()
        XCTAssertEqual(f.v, 1)
        XCTAssertEqual(f.version, "1.6.1")
        XCTAssertEqual(f.now, 1_760_000_000_000)
        XCTAssertEqual(f.lastPollAt, 1_759_999_990_000)
        XCTAssertEqual(f.threshold, 95)
        XCTAssertEqual(f.weekThreshold, 99)
        XCTAssertEqual(f.autoSwitch, true)
        XCTAssertEqual(f.accounts.map(\.id), ["a1", "a2", "a3"], "order kept")
    }

    func testDecodesUsageWithNullResetAndScoped() throws {
        let a = try Fixtures.feed().accounts[0]
        XCTAssertTrue(a.cli)
        XCTAssertFalse(a.desktop)
        XCTAssertEqual(a.plan, "Max 20x")
        XCTAssertEqual(a.usage?.five, Window(u: 9, resetsAt: 1_760_016_740_000))
        XCTAssertEqual(a.usage?.week, Window(u: 24, resetsAt: nil))
        XCTAssertEqual(a.usage?.fetchedAt, 1_759_999_990_000)
        XCTAssertEqual(a.usage?.scoped, [Scoped(name: "Fable", u: 3, resetsAt: 1_760_172_800_000)])
    }

    func testNullUsageStaysNil() throws {
        let a = try Fixtures.feed().accounts[1]
        XCTAssertNil(a.usage)
        XCTAssertNil(a.error)
    }

    func testUnknownFieldsAreIgnoredAndErrorKept() throws {
        let a = try Fixtures.feed().accounts[2]
        XCTAssertTrue(a.desktop)
        XCTAssertEqual(a.error, "token expired")
        XCTAssertNil(a.usage?.scoped, "no scoped key: nil")
    }

    func testMissingFlagsDefaultToFalse() throws {
        let json = #"{"v":1,"now":5,"accounts":[{"id":"z","email":"z@x"}]}"#
        let a = try Feed.decode(Data(json.utf8)).accounts[0]
        XCTAssertFalse(a.cli)
        XCTAssertFalse(a.desktop)
        XCTAssertNil(a.usage)
    }

    func testFractionalPercentIsRounded() throws {
        let json = #"{"v":1,"now":5,"accounts":[{"id":"z","email":"z@x","usage":{"five":{"u":9.6,"resetsAt":null},"week":{"u":0}}}]}"#
        let u = try Feed.decode(Data(json.utf8)).accounts[0].usage
        XCTAssertEqual(u?.five.u, 10)
        XCTAssertEqual(u?.week.resetsAt, nil)
    }

    func testOtherVersionIsRefused() {
        let json = #"{"v":2,"now":5,"accounts":[]}"#
        XCTAssertThrowsError(try Feed.decode(Data(json.utf8))) { err in
            XCTAssertEqual(err as? FeedError, .unsupportedVersion(2))
        }
    }

    func testGarbageIsBadData() {
        XCTAssertThrowsError(try Feed.decode(Data("not json".utf8))) { err in
            guard case .badData = err as? FeedError else { return XCTFail("\(err)") }
        }
    }
}
