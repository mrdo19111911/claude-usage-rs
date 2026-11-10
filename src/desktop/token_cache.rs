// Picking a usable token out of Claude Desktop's decrypted token cache.
// The cache maps `acct:<account-uuid>|<client-id>:<device>:<url>:<scopes>` to
// `{ token, expiresAt (ms), … }`. We want the live account's token for the Claude CLI
// OAuth client, with a usage scope, not yet expired, newest expiry first.

use serde_json::{Map, Value};

// Returns (access token, expiresAt ms) for `account_uuid` and `client_id`, or None.
pub(crate) fn pick_token(cache: &Map<String, Value>, account_uuid: &str, client_id: &str, now_ms: i64) -> Option<(String, i64)> {
    let acct = format!("acct:{account_uuid}|");
    let mut best: Option<(String, i64)> = None;
    for (key, ent) in cache {
        if !key.contains(&acct) || !key.contains(client_id) {
            continue;
        }
        if !key.contains("user:inference") && !key.contains("user:profile") {
            continue;
        }
        let Some(tok) = ent.get("token").and_then(Value::as_str).filter(|s| !s.is_empty()) else { continue };
        let exp = ent.get("expiresAt").and_then(Value::as_i64).unwrap_or(0);
        if exp <= now_ms {
            continue;
        }
        if best.as_ref().is_none_or(|(_, b)| exp > *b) {
            best = Some((tok.to_string(), exp));
        }
    }
    best
}
