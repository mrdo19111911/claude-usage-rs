// The pairing token in the Keychain (generic password, service com.smartlog.scas), readable
// after first unlock so the widget can fetch in the background. Shared with the widget through
// the App Group as keychain access group; when that group is not in the entitlements (unsigned
// simulator build) it falls back to the app's own keychain. No Security framework: no-op.
import Foundation
#if canImport(Security)
import Security
#endif

public struct KeychainVault: TokenVault {
    let service = "com.smartlog.scas"
    let account = "pairing-token"
    let group: String? = Store.appGroup

    public init() {}

    #if canImport(Security)
    public func read() -> String? {
        for g in groups {
            var q = base(g)
            q[kSecReturnData as String] = true
            q[kSecMatchLimit as String] = kSecMatchLimitOne
            var out: CFTypeRef?
            if SecItemCopyMatching(q as CFDictionary, &out) == errSecSuccess, let d = out as? Data {
                return String(data: d, encoding: .utf8)
            }
        }
        return nil
    }

    @discardableResult
    public func write(_ token: String) -> Bool {
        delete()
        for g in groups {
            var q = base(g)
            q[kSecValueData as String] = Data(token.utf8)
            q[kSecAttrAccessible as String] = kSecAttrAccessibleAfterFirstUnlock
            if SecItemAdd(q as CFDictionary, nil) == errSecSuccess { return true }
        }
        return false
    }

    public func delete() {
        for g in groups { SecItemDelete(base(g) as CFDictionary) }
    }

    /// The shared group first, then none (the app's default access group).
    var groups: [String?] { group == nil ? [nil] : [group, nil] }

    func base(_ group: String?) -> [String: Any] {
        var q: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account,
        ]
        if let group { q[kSecAttrAccessGroup as String] = group }
        return q
    }
    #else
    public func read() -> String? { nil }
    @discardableResult public func write(_ token: String) -> Bool { false }
    public func delete() {}
    #endif
}
