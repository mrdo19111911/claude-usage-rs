// /v1/messages end to end, against a fake Google on this machine: the stream arrives as
// Google sends it, the key goes only in its header, errors come back Anthropic-shaped.

use super::sse::Sse;
use super::test_app::{config, server, KEY, TOKEN};
use super::Config;
use super::upstream::TEST_BASE;
use std::io::{Read, Write};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

// Every URL the fake Google was asked for, and whether the key header came with it.
static SEEN: Mutex<Vec<(String, bool)>> = Mutex::new(vec![]);

fn chunk(text: &str) -> String {
    format!("data: {{\"candidates\":[{{\"content\":{{\"role\":\"model\",\"parts\":[{{\"text\":\"{text}\"}}]}}}}]}}\r\n\r\n")
}

const LAST: &str = "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[]},\"finishReason\":\"STOP\"}],\"usageMetadata\":{\"promptTokenCount\":5,\"candidatesTokenCount\":2}}\r\n\r\n";

fn fake_google() {
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(|| {
        let s = tiny_http::Server::http("127.0.0.1:0").unwrap();
        *TEST_BASE.lock().unwrap() = Some(format!("http://127.0.0.1:{}", s.server_addr().to_ip().unwrap().port()));
        thread::spawn(move || {
            for rq in s.incoming_requests() {
                thread::spawn(move || answer(rq));
            }
        });
    });
}

fn answer(rq: tiny_http::Request) {
    let url = rq.url().to_string();
    let keyed = rq.headers().iter().any(|h| h.field.equiv("x-goog-api-key") && h.value.as_str() == KEY);
    SEEN.lock().unwrap().push((url.clone(), keyed));
    let json = |code: u16, body: &str| tiny_http::Response::from_string(body).with_status_code(code);
    if url.contains("gemini-busy") {
        let body = r#"{"error":{"code":429,"message":"Quota exceeded","status":"RESOURCE_EXHAUSTED","details":[{"@type":"type.googleapis.com/google.rpc.RetryInfo","retryDelay":"7s"}]}}"#;
        let _ = rq.respond(json(429, body));
    } else if url.contains("gemini-bad-call") {
        let body = r#"{"candidates":[{"content":{"role":"model","parts":[]},"finishReason":"MALFORMED_FUNCTION_CALL"}]}"#;
        let _ = rq.respond(json(200, body));
    } else if url.ends_with(":generateContent") {
        let body = r#"{"candidates":[{"content":{"role":"model","parts":[{"text":"Hello"}]},"finishReason":"STOP"}],"usageMetadata":{"promptTokenCount":5,"candidatesTokenCount":1}}"#;
        let _ = rq.respond(json(200, body));
    } else {
        // Two pieces of text 600 ms apart, then the end 600 ms later.
        let mut out = Sse::open(rq).unwrap();
        out.send(&chunk("Hel")).unwrap();
        thread::sleep(Duration::from_millis(600));
        out.send(&chunk("lo")).unwrap();
        thread::sleep(Duration::from_millis(600));
        out.send(LAST).unwrap();
        out.end();
    }
}

fn ask(port: u16, model: &str, stream: bool, marks: &[&str]) -> (String, Vec<u128>) {
    let body = format!(r#"{{"model":"{model}","max_tokens":100,"stream":{stream},"messages":[{{"role":"user","content":"hi"}}]}}"#);
    let mut s = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    let head = format!("POST /v1/messages?beta=true HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAuthorization: Bearer {TOKEN}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
    s.write_all((head + &body).as_bytes()).unwrap();
    let t = Instant::now();
    let (mut all, mut at, mut buf) = (String::new(), vec![], [0u8; 4096]);
    loop {
        let n = s.read(&mut buf).expect("the answer ends with the connection");
        if n == 0 {
            return (all, at);
        }
        all.push_str(&String::from_utf8_lossy(&buf[..n]));
        while at.len() < marks.len() && all.contains(marks[at.len()]) {
            at.push(t.elapsed().as_millis());
        }
    }
}

#[test]
fn streams_each_piece_as_google_sends_it() {
    fake_google();
    let (_app, port) = server(Some(config()));
    let marks = ["message_start", "\"Hel\"", "\"lo\"", "message_stop"];
    let (all, at) = ask(port, "claude-sonnet-4-5", true, &marks);
    println!("/v1/messages through the fake Google: {marks:?} at {at:?} ms");
    assert_eq!(at.len(), 4, "{all}");
    assert!(all.starts_with("HTTP/1.1 200 OK\r\n") && all.contains("text/event-stream"), "{all}");
    assert!(at[1] < 400, "first text at {} ms", at[1]);
    assert!(at[2] >= 550 && at[2] < 1000, "second text at {} ms", at[2]);
    assert!(at[3] >= 1150, "end at {} ms", at[3]);
    assert!(all.contains("end_turn"));
    let seen = SEEN.lock().unwrap();
    // Other tests share the fake Google: find this one's call by its URL.
    let url = format!("/models/{}:streamGenerateContent?alt=sse", super::MODEL);
    let (_, keyed) = seen.iter().find(|(u, _)| *u == url).expect("asked for the main model, streamed");
    assert!(keyed, "the key in its header");
    assert!(seen.iter().all(|(u, _)| !u.contains(KEY)), "never in a URL");
}

#[test]
fn answers_in_one_message_without_stream() {
    fake_google();
    let (_app, port) = server(Some(config()));
    let (all, _) = ask(port, "claude-haiku-4-5", false, &[]);
    assert!(all.starts_with("HTTP/1.1 200"), "{all}");
    let body: serde_json::Value = serde_json::from_str(all.split("\r\n\r\n").nth(1).unwrap()).unwrap();
    assert_eq!(body["type"], "message");
    assert_eq!(body["content"][0]["text"], "Hello");
    let seen = SEEN.lock().unwrap();
    assert!(seen.iter().any(|(u, _)| u == &format!("/models/{}:generateContent", super::FAST_MODEL)), "haiku → the fast model");
}

#[test]
fn a_google_429_is_a_rate_limit_with_retry_after() {
    fake_google();
    let (_app, port) = server(Some(config()));
    let (all, _) = ask(port, "gemini-busy", true, &[]);
    assert!(all.starts_with("HTTP/1.1 429"), "{all}");
    assert!(all.to_lowercase().contains("retry-after: 7\r\n"), "{all}");
    assert!(all.contains("rate_limit_error"));
    assert!(!all.contains(KEY));
}

#[test]
fn a_bad_function_call_without_stream_is_a_500() {
    fake_google();
    let (_app, port) = server(Some(config()));
    let (all, _) = ask(port, "gemini-bad-call", false, &[]);
    assert!(all.starts_with("HTTP/1.1 500"), "{all}");
    assert!(all.contains("\"type\":\"error\""), "{all}");
}

#[test]
fn a_busy_model_hands_over_to_the_other_one_once() {
    fake_google();
    let cfg = Config { model: "gemini-busy-main".into(), ..config() };
    let (_app, port) = server(Some(cfg));
    let (all, _) = ask(port, "claude-sonnet-4-5", true, &[]);
    assert!(all.starts_with("HTTP/1.1 200"), "{all}");
    assert!(all.contains(&format!("\"model\":\"{}\"", super::FAST_MODEL)), "says which model answered: {all}");
    // Both busy: Google's own error, after one try each.
    let cfg = Config { model: "gemini-busy-a".into(), fast_model: "gemini-busy-b".into(), ..config() };
    let (_app, port) = server(Some(cfg));
    let (all, _) = ask(port, "claude-sonnet-4-5", true, &[]);
    assert!(all.starts_with("HTTP/1.1 429"), "{all}");
    let seen = SEEN.lock().unwrap();
    assert_eq!(seen.iter().filter(|(u, _)| u.contains("gemini-busy-a") || u.contains("gemini-busy-b")).count(), 2);
}
