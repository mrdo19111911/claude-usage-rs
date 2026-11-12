// The GET with the token, and how answers and failures map to FeedError.
import XCTest
@testable import SCASKit

final class FeedClientTests: XCTestCase {
    let pairing = Pairing(host: "10.0.0.2", port: 4778, token: "tok")

    func testRequestCarriesTheToken() {
        let r = FeedClient.request(for: pairing)
        XCTAssertEqual(r.httpMethod, "GET")
        XCTAssertEqual(r.url?.absoluteString, "http://10.0.0.2:4778/api/mobile/v1/state")
        XCTAssertEqual(r.value(forHTTPHeaderField: "Authorization"), "Bearer tok")
        XCTAssertEqual(r.value(forHTTPHeaderField: "Accept"), "application/json")
        XCTAssertLessThanOrEqual(r.timeoutInterval, 10)
    }

    func test200DecodesTheFeed() throws {
        let f = try FeedClient.read(status: 200, data: Data(Fixtures.feedJSON.utf8))
        XCTAssertEqual(f.accounts.count, 3)
    }

    func testStatusesMapToErrors() {
        let cases: [(Int, FeedError)] = [(401, .unauthorized), (403, .forbidden), (404, .badStatus(404)),
                                         (500, .badStatus(500))]
        for (code, want) in cases {
            XCTAssertThrowsError(try FeedClient.read(status: code, data: Data())) { err in
                XCTAssertEqual(err as? FeedError, want, "status \(code)")
            }
        }
    }

    func testBadBodyIsBadData() {
        XCTAssertThrowsError(try FeedClient.read(status: 200, data: Data("<html>".utf8))) { err in
            guard case .badData = err as? FeedError else { return XCTFail("\(err)") }
        }
    }

    func testNetworkErrorsAreUnreachable() {
        let e = FeedClient.map(URLError(.cannotConnectToHost))
        guard case .unreachable = e else { return XCTFail("\(e)") }
        XCTAssertEqual(FeedClient.map(FeedError.forbidden), .forbidden, "FeedError passes through")
    }
}
