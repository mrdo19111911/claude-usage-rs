// The pairing text `scas://pair?host=..&port=..&token=..` (docs/specs/mobile-feed.md).
import XCTest
@testable import SCASKit

final class PairingTests: XCTestCase {
    let token = "AbCdEfGhIjKlMnOpQrStUvWxYz0123456789-_abcde" // 43 chars, base64url

    func testParsesTheDashboardText() throws {
        let p = try XCTUnwrap(Pairing.parse("scas://pair?host=192.168.1.5&port=4778&token=\(token)"))
        XCTAssertEqual(p, Pairing(host: "192.168.1.5", port: 4778, token: token))
        XCTAssertEqual(p.baseURL.absoluteString, "http://192.168.1.5:4778")
    }

    func testRoundTrip() {
        let p = Pairing(host: "100.101.102.103", port: 4778, token: token)
        XCTAssertEqual(Pairing.parse(p.text), p)
    }

    func testOrderOfPartsDoesNotMatterAndSpacesAreTrimmed() {
        let p = Pairing.parse("  scas://pair?token=t1&port=80&host=10.0.0.2\n")
        XCTAssertEqual(p, Pairing(host: "10.0.0.2", port: 80, token: "t1"))
    }

    func testIPv6HostGetsBrackets() throws {
        let p = try XCTUnwrap(Pairing.parse("scas://pair?host=fd7a:115c:a1e0::1&port=4778&token=t"))
        XCTAssertEqual(p.host, "fd7a:115c:a1e0::1")
        XCTAssertEqual(p.baseURL.absoluteString, "http://[fd7a:115c:a1e0::1]:4778")
        XCTAssertEqual(Pairing.parse(p.text), p)
    }

    func testRejectsWrongScheme() {
        XCTAssertNil(Pairing.parse("http://pair?host=10.0.0.2&port=4778&token=t"))
        XCTAssertNil(Pairing.parse("scas://other?host=10.0.0.2&port=4778&token=t"))
    }

    func testRejectsMissingOrEmptyParts() {
        XCTAssertNil(Pairing.parse("scas://pair?host=10.0.0.2&port=4778"))
        XCTAssertNil(Pairing.parse("scas://pair?host=10.0.0.2&port=4778&token="))
        XCTAssertNil(Pairing.parse("scas://pair?port=4778&token=t"))
        XCTAssertNil(Pairing.parse("scas://pair?host=10.0.0.2&token=t"))
        XCTAssertNil(Pairing.parse(""))
    }

    func testRejectsBadPorts() {
        XCTAssertNil(Pairing.parse("scas://pair?host=h&port=x&token=t"))
        XCTAssertNil(Pairing.parse("scas://pair?host=h&port=0&token=t"))
        XCTAssertNil(Pairing.parse("scas://pair?host=h&port=70000&token=t"))
    }

    func testStateURL() {
        let p = Pairing(host: "10.0.0.2", port: 4778, token: "t")
        XCTAssertEqual(p.stateURL.absoluteString, "http://10.0.0.2:4778/api/mobile/v1/state")
    }
}
