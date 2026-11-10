use super::token_cache::pick_token;
use serde_json::{json, Map, Value};

const ACCT: &str = "d8828315-6029-43d2-868e-056dd7fac733";
const OTHER: &str = "11111111-2222-3333-4444-555555555555";
const CLIENT: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";
const NOW: i64 = 1_762_000_000_000; // ~2025

fn key(acct: &str, client: &str, scopes: &str) -> String {
    format!("acct:{acct}|{client}:dev:https://api.anthropic.com:{scopes}")
}

fn cache(entries: &[(String, Value)]) -> Map<String, Value> {
    entries.iter().cloned().collect()
}

#[test]
fn picks_the_claude_cli_clients_usage_token() {
    let c = cache(&[(key(ACCT, CLIENT, "user:inference user:profile"), json!({ "token": "good", "expiresAt": NOW + 1000 }))]);
    assert_eq!(pick_token(&c, ACCT, CLIENT, NOW), Some(("good".into(), NOW + 1000)));
}

#[test]
fn skips_another_account_another_client_and_a_scopeless_entry() {
    let c = cache(&[
        (key(OTHER, CLIENT, "user:profile"), json!({ "token": "wrong-acct", "expiresAt": NOW + 9000 })),
        (key(ACCT, "a473d7bb-17ac-43a7-abc0-a1343d7c2805", "user:profile"), json!({ "token": "wrong-client", "expiresAt": NOW + 9000 })),
        (key(ACCT, CLIENT, "user:sessions:claude_code"), json!({ "token": "no-usage-scope", "expiresAt": NOW + 9000 })),
    ]);
    assert_eq!(pick_token(&c, ACCT, CLIENT, NOW), None);
}

#[test]
fn skips_expired_and_empty_tokens() {
    let c = cache(&[
        (key(ACCT, CLIENT, "user:inference"), json!({ "token": "stale", "expiresAt": NOW - 1 })),
        (key(ACCT, CLIENT, "user:profile"), json!({ "token": "", "expiresAt": NOW + 9000 })),
    ]);
    assert_eq!(pick_token(&c, ACCT, CLIENT, NOW), None);
}

#[test]
fn latest_expiry_wins() {
    let c = cache(&[
        (key(ACCT, CLIENT, "user:inference user:profile"), json!({ "token": "soon", "expiresAt": NOW + 1000 })),
        (key(ACCT, CLIENT, "user:inference user:file_upload user:profile"), json!({ "token": "later", "expiresAt": NOW + 5000 })),
    ]);
    assert_eq!(pick_token(&c, ACCT, CLIENT, NOW), Some(("later".into(), NOW + 5000)));
}
