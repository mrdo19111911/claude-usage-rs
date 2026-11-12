// GET http://host:port/api/mobile/v1/state with `Authorization: Bearer <token>`.
// In: a Pairing. Out: (Feed, raw body) or FeedError: 401 unauthorized, 403 forbidden,
// other non-200 badStatus, no answer unreachable, unreadable body badData.
import Foundation

/// Where figures come from (FeedClient; a stub in tests).
public protocol FeedSource {
    func fetch(_ pairing: Pairing) async throws -> (Feed, Data)
}

public struct FeedClient: FeedSource {
    let session: URLSession

    public init(session: URLSession = .shared) { self.session = session }

    public func fetch(_ pairing: Pairing) async throws -> (Feed, Data) {
        do {
            let (data, response) = try await session.data(for: Self.request(for: pairing))
            let status = (response as? HTTPURLResponse)?.statusCode ?? 0
            return (try Self.read(status: status, data: data), data)
        } catch {
            throw Self.map(error)
        }
    }

    static func request(for p: Pairing) -> URLRequest {
        var r = URLRequest(url: p.stateURL, cachePolicy: .reloadIgnoringLocalCacheData, timeoutInterval: 8)
        r.httpMethod = "GET"
        r.setValue("Bearer \(p.token)", forHTTPHeaderField: "Authorization")
        r.setValue("application/json", forHTTPHeaderField: "Accept")
        return r
    }

    static func read(status: Int, data: Data) throws -> Feed {
        switch status {
        case 200: return try Feed.decode(data)
        case 401: throw FeedError.unauthorized
        case 403: throw FeedError.forbidden
        default: throw FeedError.badStatus(status)
        }
    }

    static func map(_ error: Error) -> FeedError {
        if let e = error as? FeedError { return e }
        if let e = error as? URLError { return .unreachable(e.localizedDescription) }
        return .unreachable(String(describing: error))
    }
}
