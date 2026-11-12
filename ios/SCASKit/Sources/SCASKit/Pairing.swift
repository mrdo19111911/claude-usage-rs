// The pairing text the PC's dashboard shows (QR and text):
// `scas://pair?host=<ip>&port=<port>&token=<token>` (docs/specs/mobile-feed.md).
// In: that text (spaces around it allowed, parts in any order). Out: Pairing, or nil when the
// scheme is not scas, the path is not pair, a part is missing/empty or the port is not 1...65535.
import Foundation

public struct Pairing: Equatable, Codable, Sendable {
    public var host: String
    public var port: Int
    public var token: String

    public init(host: String, port: Int, token: String) {
        self.host = host; self.port = port; self.token = token
    }

    public static func parse(_ text: String) -> Pairing? {
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard let parts = URLComponents(string: trimmed),
              parts.scheme?.lowercased() == "scas",
              parts.host?.lowercased() == "pair" else { return nil }
        let items = parts.queryItems ?? []
        func value(_ name: String) -> String? {
            guard let v = items.first(where: { $0.name == name })?.value, !v.isEmpty else { return nil }
            return v
        }
        guard let host = value("host"), let token = value("token"),
              let portText = value("port"), let port = Int(portText),
              (1...65_535).contains(port) else { return nil }
        return Pairing(host: host, port: port, token: token)
    }

    /// The pairing text again (what `parse` reads).
    public var text: String {
        var parts = URLComponents()
        parts.scheme = "scas"
        parts.host = "pair"
        parts.queryItems = [
            URLQueryItem(name: "host", value: host),
            URLQueryItem(name: "port", value: String(port)),
            URLQueryItem(name: "token", value: token),
        ]
        return parts.string ?? "scas://pair?host=\(host)&port=\(port)&token=\(token)"
    }

    /// `http://host:port`; an IPv6 host goes in brackets.
    public var baseURL: URL {
        let h = host.contains(":") && !host.hasPrefix("[") ? "[\(host)]" : host
        return URL(string: "http://\(h):\(port)")!
    }

    /// The feed's one route.
    public var stateURL: URL {
        URL(string: baseURL.absoluteString + "/api/mobile/v1/state")!
    }
}
