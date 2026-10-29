// Signing in to a Claude account in the browser, the way Claude Code's /login does it
// (OAuth 2 with PKCE; values read from Claude Code 2.1.308): the browser goes to
// claude.com's authorize page, comes back to http://localhost:<port>/callback with a
// code, and the code is exchanged for tokens. The tokens are the dashboard's own: the
// login Claude Code uses now is not touched.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub const AUTHORIZE_URL: &str = "https://claude.com/cai/oauth/authorize";
pub const SCOPES: &str = "org:create_api_key user:profile user:inference user:sessions:claude_code user:mcp_servers user:file_upload";
/// A sign-in left this long is dropped.
pub const PENDING_MS: i64 = 15 * 60 * 1000;

pub struct Pending {
    pub state: String,
    pub verifier: String,
    pub started: i64,
}

fn random_b64(bytes: usize) -> String {
    let mut b = vec![0u8; bytes];
    getrandom::getrandom(&mut b).expect("system random numbers");
    URL_SAFE_NO_PAD.encode(b)
}

pub fn challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

pub fn redirect_uri(port: u16) -> String {
    format!("http://localhost:{port}/callback")
}

pub fn start(now: i64) -> Pending {
    Pending { state: random_b64(32), verifier: random_b64(32), started: now }
}

pub fn authorize_url(p: &Pending, client_id: &str, port: u16) -> String {
    let q = [
        ("code", "true".to_string()),
        ("client_id", client_id.to_string()),
        ("response_type", "code".to_string()),
        ("redirect_uri", redirect_uri(port)),
        ("scope", SCOPES.to_string()),
        ("code_challenge", challenge(&p.verifier)),
        ("code_challenge_method", "S256".to_string()),
        ("state", p.state.clone()),
    ];
    let q: Vec<String> = q.iter().map(|(k, v)| format!("{k}={}", encode(v))).collect();
    format!("{AUTHORIZE_URL}?{}", q.join("&"))
}

pub fn exchange_body(p: &Pending, code: &str, client_id: &str, port: u16) -> Value {
    json!({
        "grant_type": "authorization_code", "code": code, "redirect_uri": redirect_uri(port),
        "client_id": client_id, "code_verifier": p.verifier, "state": p.state,
    })
}

/// Claude Code's own shape of a login (.credentials.json's claudeAiOauth).
pub fn creds(token: &Value, profile: &Value, now: i64) -> Value {
    let org = &profile["organization"];
    let sub = org["organization_type"].as_str().unwrap_or("").trim_start_matches("claude_").to_string();
    let scopes: Vec<&str> = token["scope"].as_str().unwrap_or(SCOPES).split_whitespace().collect();
    json!({
        "accessToken": token["access_token"],
        "refreshToken": token["refresh_token"],
        "expiresAt": now + token["expires_in"].as_i64().unwrap_or(3600) * 1000,
        "scopes": scopes,
        "subscriptionType": if sub.is_empty() { Value::Null } else { json!(sub) },
        "rateLimitTier": org["rate_limit_tier"],
    })
}

/// ~/.claude.json's oauthAccount for this account, written when it is put to use.
pub fn oauth_account(profile: &Value) -> Value {
    let (a, o) = (&profile["account"], &profile["organization"]);
    json!({
        "accountUuid": a["uuid"], "emailAddress": a["email"], "organizationUuid": o["uuid"],
        "hasExtraUsageEnabled": o["has_extra_usage_enabled"], "billingType": o["billing_type"],
        "accountCreatedAt": a["created_at"], "subscriptionCreatedAt": o["subscription_created_at"],
        "seatTier": o["seat_tier"], "organizationName": o["name"], "organizationType": o["organization_type"],
        "organizationRateLimitTier": o["rate_limit_tier"], "displayName": a["display_name"], "fullName": a["full_name"],
    })
}

pub fn encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

pub fn decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = vec![];
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < b.len() => {
                match std::str::from_utf8(&b[i + 1..i + 3]).ok().and_then(|h| u8::from_str_radix(h, 16).ok()) {
                    Some(v) => {
                        out.push(v);
                        i += 2;
                    }
                    None => out.push(b'%'),
                }
            }
            c => out.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// One parameter of a URL's query.
pub fn param(url: &str, name: &str) -> Option<String> {
    url.split_once('?')?.1.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=').unwrap_or((kv, ""));
        (k == name).then(|| decode(v))
    })
}
