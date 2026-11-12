// In use / switch to / used up, as the tray widget's split_others (docs/specs/modules.md).
import XCTest
@testable import SCASKit

final class SplitTests: XCTestCase {
    typealias A = Fixtures

    func testInUseKeepsFeedOrderAndBothSides() {
        let s = Split.of([A.account("d", five: 1, desktop: true), A.account("o", five: 2),
                          A.account("c", five: 3, cli: true)])
        XCTAssertEqual(s.inUse.map(\.id), ["d", "c"])
        XCTAssertEqual(s.switchTo.map(\.id), ["o"])
    }

    func testOneAccountOnBothSidesIsOneRow() {
        let s = Split.of([A.account("b", five: 1, cli: true, desktop: true)])
        XCTAssertEqual(s.inUse.map(\.id), ["b"])
    }

    func testSwitchToMostRoomFirstNoFiguresLast() {
        let s = Split.of([
            A.account("none"),
            A.account("busy", five: 80, week: 10),
            A.account("weekly", five: 5, week: 60),
            A.account("free", five: 5, week: 20),
            A.account("tie", five: 20, week: 20),
        ])
        // key: (max(five, week), week): free (20,20) and tie (20,20) by email, weekly 60, busy 80
        XCTAssertEqual(s.switchTo.map(\.id), ["free", "tie", "weekly", "busy", "none"])
    }

    func testFullIsEitherWindowAtOrOver100() {
        let s = Split.of([A.account("five", five: 100, week: 1), A.account("week", five: 1, week: 100),
                          A.account("ok", five: 99, week: 99)])
        XCTAssertEqual(s.switchTo.map(\.id), ["ok"])
        XCTAssertEqual(Set(s.usedUp.map(\.account.id)), ["five", "week"])
    }

    func testUsedUpSoonestFreeFirstUnknownLast() {
        let n = A.now
        let s = Split.of([
            A.account("late", five: 100, fiveReset: n + 5 * A.hour),
            A.account("unknown", five: 100),
            A.account("soon", five: 100, fiveReset: n + A.hour),
        ])
        XCTAssertEqual(s.usedUp.map(\.account.id), ["soon", "late", "unknown"])
        XCTAssertEqual(s.usedUp.map(\.freeAt), [n + A.hour, n + 5 * A.hour, nil])
    }

    func testFreeAtIsTheLatestResetOfTheFullWindows() {
        let n = A.now
        let both = A.account("x", five: 100, week: 100, fiveReset: n + A.hour, weekReset: n + 50 * A.hour)
        XCTAssertEqual(Split.freeAt(both.usage!), .some(n + 50 * A.hour))
        let onlyFive = A.account("y", five: 100, week: 3, fiveReset: n + A.hour, weekReset: n + 50 * A.hour)
        XCTAssertEqual(Split.freeAt(onlyFive.usage!), .some(n + A.hour))
        let notFull = A.account("z", five: 3)
        XCTAssertNil(Split.freeAt(notFull.usage!))
        let fullNoReset = A.account("w", five: 100, week: 100, fiveReset: n + A.hour)
        XCTAssertEqual(Split.freeAt(fullNoReset.usage!), .some(nil), "full, time not known")
    }

    func testInUseAccountsAreNotRepeatedBelow() {
        let s = Split.of([A.account("c", five: 100, fiveReset: A.now + A.hour, cli: true)])
        XCTAssertEqual(s.inUse.map(\.id), ["c"])
        XCTAssertTrue(s.switchTo.isEmpty)
        XCTAssertTrue(s.usedUp.isEmpty)
    }

    func testFromTheSampleFeed() throws {
        let s = Split.of(try Fixtures.feed().accounts)
        XCTAssertEqual(s.inUse.map(\.id), ["a1", "a3"])
        XCTAssertEqual(s.switchTo.map(\.id), ["a2"])
        XCTAssertEqual(s.best?.id, "a2")
        XCTAssertEqual(s.main?.id, "a1", "Claude Code's account first")
    }
}
