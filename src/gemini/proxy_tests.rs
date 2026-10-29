// /v1/*: the token check, the routes, and the way in through server.rs.

use super::proxy::{route, same, token_ok, Route};
use super::test_app::{call, config, server, TOKEN};
use std::io::{Read, Write};

#[test]
fn compares_tokens_whole() {
    assert!(same("abc", "abc"));
    assert!(!same("abc", "abd"));
    assert!(!same("abc", "ab"), "a prefix is not enough");
    assert!(!same("", "abc"));
}

#[test]
fn takes_the_token_from_either_header() {
    let t = "tok-1";
    assert!(token_ok(Some("Bearer tok-1"), None, None, t));
    assert!(token_ok(Some("bearer tok-1"), None, None, t), "scheme in any case");
    assert!(token_ok(None, Some("tok-1"), None, t));
    assert!(token_ok(Some("Bearer wrong"), Some("tok-1"), None, t), "either one will do");
    assert!(!token_ok(Some("Bearer wrong"), None, None, t));
    assert!(!token_ok(Some("tok-1"), None, None, t), "Authorization needs Bearer");
    assert!(!token_ok(None, None, None, t));
    assert!(!token_ok(Some("Bearer "), Some(""), None, ""), "no token saved: nothing opens");
    // Claude Code signed in to claude.ai sends its own login in Authorization.
    assert!(token_ok(Some("Bearer sk-ant-oat-login"), None, Some("tok-1"), t), "x-cud-token will do");
    assert!(!token_ok(Some("Bearer sk-ant-oat-login"), None, Some("wrong"), t));
}

#[test]
fn routes_by_method_and_path() {
    assert_eq!(route("POST", "/v1/messages"), Route::Messages);
    assert_eq!(route("POST", "/v1/messages/count_tokens"), Route::CountTokens);
    assert_eq!(route("GET", "/v1/models"), Route::Models);
    assert_eq!(route("GET", "/v1/ping"), Route::Ping);
    assert_eq!(route("GET", "/v1/messages"), Route::NotFound);
    assert_eq!(route("POST", "/v1/complete"), Route::NotFound);
}

#[test]
fn ping_needs_no_token() {
    let (_app, port) = server(None);
    assert_eq!(call(port, "GET", "/v1/ping", &[], None), (200, "{}".into()));
}

#[test]
fn no_key_saved_is_401_in_anthropic_shape() {
    let (_app, port) = server(None);
    let (code, body) = call(port, "POST", "/v1/messages?beta=true", &[("Authorization", "Bearer x")], Some("{}"));
    assert_eq!(code, 401);
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["type"], "error");
    assert_eq!(v["error"]["type"], "authentication_error");
}

#[test]
fn wrong_or_missing_token_is_401_without_the_page_header() {
    let (_app, port) = server(Some(config()));
    // No X-CUD: the page's write check must not answer (403) for /v1.
    assert_eq!(call(port, "POST", "/v1/messages", &[], Some("{}")).0, 401);
    assert_eq!(call(port, "POST", "/v1/messages", &[("x-api-key", "nope")], Some("{}")).0, 401);
    assert_eq!(call(port, "GET", "/v1/models", &[("Authorization", "Bearer nope")], None).0, 401);
}

#[test]
fn routes_with_the_token() {
    let (_app, port) = server(Some(config()));
    let bearer = format!("Bearer {TOKEN}");
    assert_eq!(call(port, "GET", "/v1/models?limit=1000", &[("Authorization", &bearer)], None), (200, r#"{"data":[]}"#.into()));
    assert_eq!(call(port, "GET", "/v1/models", &[("x-api-key", TOKEN)], None).0, 200);
    let (code, body) = call(port, "GET", "/v1/nothing", &[("x-api-key", TOKEN)], None);
    assert_eq!(code, 404);
    assert!(body.contains("not_found_error"), "{body}");
    let (code, body) = call(port, "POST", "/v1/messages", &[("x-api-key", TOKEN)], Some("not json"));
    assert_eq!(code, 400);
    assert!(body.contains("invalid_request_error"), "{body}");
}

#[test]
fn other_host_names_are_refused() {
    let (_app, port) = server(Some(config()));
    let mut s = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    let rq = format!("GET /v1/ping HTTP/1.1\r\nHost: evil.example:{port}\r\nConnection: close\r\n\r\n");
    s.write_all(rq.as_bytes()).unwrap();
    let mut out = String::new();
    s.read_to_string(&mut out).unwrap();
    assert!(out.starts_with("HTTP/1.1 403"), "{out}");
}
