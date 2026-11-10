// Reading Claude Desktop's own access token for the live account's usage figures
// (docs/specs/desktop-token.md). Windows only; read-only; never refreshed or written back.

use super::{read_config, safe_storage, token_cache};
use serde_json::{json, Value};
use std::path::Path;

pub(crate) fn desktop_access_token(dir: &Path, account_uuid: &str) -> Option<Value> {
    let key = safe_storage::app_key(dir)?;
    let config = read_config(dir).ok()?;
    let now_ms = crate::now();
    let mut best: Option<(String, i64)> = None;
    // The newer cache first; take the latest-expiring token across both.
    for name in ["oauth:tokenCacheV2", "oauth:tokenCache"] {
        let Some(enc) = config.get(name).and_then(Value::as_str) else { continue };
        let Some(plain) = safe_storage::decrypt_v10(&key, enc) else { continue };
        let Ok(Value::Object(cache)) = serde_json::from_slice::<Value>(&plain) else { continue };
        if let Some((tok, exp)) = token_cache::pick_token(&cache, account_uuid, crate::anthropic::CLIENT_ID, now_ms) {
            if best.as_ref().is_none_or(|(_, b)| exp > *b) {
                best = Some((tok, exp));
            }
        }
    }
    best.map(|(tok, exp)| json!({ "accessToken": tok, "expiresAt": exp }))
}
