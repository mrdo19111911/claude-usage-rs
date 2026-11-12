// Words and numbers the user reads; mirrors the tray widget (docs/specs/modules.md, Widget v2).
import XCTest
@testable import SCASKit

final class TextsTests: XCTestCase {
    let now = Fixtures.now // Thu 08:53:20 UTC
    let tz = Fixtures.utc
    let h = Fixtures.hour, m = Fixtures.minute

    func testTimeLeft() {
        XCTAssertEqual(Texts.left(4 * h + 39 * m), "4h39m")
        XCTAssertEqual(Texts.left(2 * h + 5 * m), "2h05m")
        XCTAssertEqual(Texts.left(51 * h), "2d 3h")
        XCTAssertEqual(Texts.left(30_000), "1m", "rounds up to the minute")
        XCTAssertEqual(Texts.left(12 * m), "12m")
        XCTAssertEqual(Texts.left(-5), "0m")
    }

    func testClockSameDayAndOtherDay() {
        XCTAssertEqual(Texts.clock(now + 4 * h + 39 * m, now: now, tz: tz), "13:32")
        XCTAssertEqual(Texts.clock(now + 48 * h, now: now, tz: tz), "Sat 08:53")
    }

    func testClockFollowsTheTimeZone() {
        let hcm = TimeZone(secondsFromGMT: 7 * 3600)!
        XCTAssertEqual(Texts.clock(now, now: now, tz: hcm), "15:53")
    }

    func testResets() {
        XCTAssertEqual(Texts.resets(now + 4 * h + 39 * m, now: now, tz: tz), "resets in 4h39m · 13:32")
        XCTAssertEqual(Texts.resets(nil, now: now, tz: tz), "not started yet")
    }

    func testPercentAndFigures() {
        XCTAssertEqual(Texts.percent(9), "9%")
        XCTAssertEqual(Texts.percent(100), "100%")
        let u = Usage(five: Window(u: 26, resetsAt: nil), week: Window(u: 74, resetsAt: nil),
                      fetchedAt: nil, scoped: nil)
        XCTAssertEqual(Texts.figures(u), "26% · 74%")
        XCTAssertEqual(Texts.figures(nil), "—")
    }

    func testBarFractionIsClamped() {
        XCTAssertEqual(Texts.fraction(50), 0.5, accuracy: 1e-9)
        XCTAssertEqual(Texts.fraction(130), 1)
        XCTAssertEqual(Texts.fraction(-3), 0)
    }

    func testAvailableAt() {
        XCTAssertEqual(Texts.availableAt(now + h, now: now, tz: tz), "09:53")
        XCTAssertEqual(Texts.availableAt(nil, now: now, tz: tz), "not known")
    }

    func testScopedChip() {
        XCTAssertEqual(Texts.chip(Scoped(name: "Fable", u: 3, resetsAt: nil)), "Fable 3%")
    }

    func testLastSeen() {
        XCTAssertEqual(Texts.lastSeen(now - 2 * h, now: now, tz: tz),
                       "Last seen 06:53 · PC not reachable (same Wi-Fi or Tailscale needed)")
        XCTAssertEqual(Texts.lastSeen(nil, now: now, tz: tz),
                       "PC not reachable (same Wi-Fi or Tailscale needed)")
    }

    func testSectionTitles() {
        XCTAssertEqual(Texts.inUse, "In use now")
        XCTAssertEqual(Texts.switchTo, "Switch to")
        XCTAssertEqual(Texts.usedUp, "Used up · available at")
        XCTAssertEqual(Texts.notChecked, "Usage not checked yet")
    }

    func testErrorWords() {
        XCTAssertEqual(Texts.problem(.unauthorized), "The PC refused the pairing. Pair again.")
        XCTAssertEqual(Texts.problem(.unreachable("x")), "PC not reachable (same Wi-Fi or Tailscale needed)")
    }
}
