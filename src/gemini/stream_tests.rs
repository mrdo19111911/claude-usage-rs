// Gemini chunks → Anthropic SSE events, in Anthropic's order and shapes.

use super::*;

// The (event, data) pairs of an SSE text, checking each event name matches data.type.
fn events(sse: &str) -> Vec<(String, Value)> {
    sse.split("\n\n").filter(|e| !e.is_empty()).map(|e| {
        let (ev, data) = e.split_once('\n').unwrap();
        let ev = ev.strip_prefix("event: ").unwrap().to_string();
        let data: Value = serde_json::from_str(data.strip_prefix("data: ").unwrap()).unwrap();
        assert_eq!(data["type"], ev.as_str());
        (ev, data)
    }).collect()
}

fn parts(p: Value) -> Value {
    json!({ "candidates": [{ "content": { "role": "model", "parts": p } }] })
}

fn last(p: Value, reason: &str) -> Value {
    json!({ "candidates": [{ "content": { "role": "model", "parts": p }, "finishReason": reason }],
        "usageMetadata": { "promptTokenCount": 10, "cachedContentTokenCount": 4, "candidatesTokenCount": 5, "thoughtsTokenCount": 2, "totalTokenCount": 21 } })
}

fn run(chunks: &[Value], sigs: &Sigs) -> Vec<(String, Value)> {
    let mut t = Translator::new("gemini-3.8-flash");
    let mut out = String::new();
    for c in chunks {
        out += &t.chunk(c, sigs);
    }
    out += &t.finish();
    events(&out)
}

fn names(ev: &[(String, Value)]) -> Vec<&str> {
    ev.iter().map(|(n, _)| n.as_str()).collect()
}

#[test]
fn start_sends_message_start_and_ping() {
    let ev = events(&Translator::new("gemini-3.8-flash").start());
    assert_eq!(names(&ev), ["message_start", "ping"]);
    let m = &ev[0].1["message"];
    assert!(m["id"].as_str().unwrap().starts_with("msg_"));
    let mut m = m.clone();
    m["id"] = json!("x");
    assert_eq!(m, json!({ "id": "x", "type": "message", "role": "assistant", "model": "gemini-3.8-flash", "content": [],
        "stop_reason": null, "stop_sequence": null,
        "usage": { "input_tokens": 0, "output_tokens": 0, "cache_creation_input_tokens": 0, "cache_read_input_tokens": 0 } }));
}

#[test]
fn text_only() {
    let ev = run(&[parts(json!([{ "text": "Hel" }])), last(json!([{ "text": "lo" }]), "STOP")], &Sigs::default());
    assert_eq!(names(&ev), ["content_block_start", "content_block_delta", "content_block_delta", "content_block_stop", "message_delta", "message_stop"]);
    assert_eq!(ev[0].1, json!({ "type": "content_block_start", "index": 0, "content_block": { "type": "text", "text": "" } }));
    assert_eq!(ev[1].1, json!({ "type": "content_block_delta", "index": 0, "delta": { "type": "text_delta", "text": "Hel" } }));
    assert_eq!(ev[3].1, json!({ "type": "content_block_stop", "index": 0 }));
    assert_eq!(ev[4].1, json!({ "type": "message_delta", "delta": { "stop_reason": "end_turn", "stop_sequence": null },
        "usage": { "input_tokens": 6, "output_tokens": 7, "cache_creation_input_tokens": 0, "cache_read_input_tokens": 4 } }));
    assert_eq!(ev[5].1, json!({ "type": "message_stop" }));
}

#[test]
fn text_then_a_function_call() {
    let sigs = Sigs::default();
    let call = json!({ "functionCall": { "name": "Read", "args": { "path": "a.txt" } }, "thoughtSignature": "SIG1" });
    let ev = run(&[parts(json!([{ "text": "Let me look" }])), last(json!([call]), "STOP")], &sigs);
    assert_eq!(names(&ev), ["content_block_start", "content_block_delta", "content_block_stop",
        "content_block_start", "content_block_delta", "content_block_stop", "message_delta", "message_stop"]);
    let start = &ev[3].1;
    assert_eq!(start["index"], 1);
    let id = start["content_block"]["id"].as_str().unwrap();
    assert!(id.len() == 30 && id.starts_with("toolu_") && id[6..].chars().all(|c| c.is_ascii_hexdigit()), "{id}");
    assert_eq!(start["content_block"], json!({ "type": "tool_use", "id": id, "name": "Read", "input": {} }));
    assert_eq!(ev[4].1, json!({ "type": "content_block_delta", "index": 1, "delta": { "type": "input_json_delta", "partial_json": "{\"path\":\"a.txt\"}" } }));
    assert_eq!(ev[6].1["delta"]["stop_reason"], "tool_use");
    assert_eq!(sigs.get(id).as_deref(), Some("SIG1"));
}

#[test]
fn thoughts_become_a_thinking_block_with_its_signature() {
    let ev = run(&[
        parts(json!([{ "text": "Pondering", "thought": true }])),
        parts(json!([{ "text": " more", "thought": true, "thoughtSignature": "TSIG" }])),
        last(json!([{ "text": "Answer" }]), "STOP"),
    ], &Sigs::default());
    assert_eq!(names(&ev), ["content_block_start", "content_block_delta", "content_block_delta", "content_block_delta", "content_block_stop",
        "content_block_start", "content_block_delta", "content_block_stop", "message_delta", "message_stop"]);
    assert_eq!(ev[0].1["content_block"], json!({ "type": "thinking", "thinking": "" }));
    assert_eq!(ev[1].1["delta"], json!({ "type": "thinking_delta", "thinking": "Pondering" }));
    assert_eq!(ev[3].1["delta"], json!({ "type": "signature_delta", "signature": "TSIG" }));
    assert_eq!(ev[5].1["index"], 1);
}

#[test]
fn a_signature_in_an_empty_text_part_goes_to_the_call() {
    let sigs = Sigs::default();
    let ev = run(&[
        parts(json!([{ "text": "", "thoughtSignature": "EMPTY_SIG" }])),
        last(json!([{ "functionCall": { "name": "Bash", "args": {} } }]), "STOP"),
    ], &sigs);
    assert_eq!(names(&ev)[0], "content_block_start");
    assert_eq!(ev[0].1["content_block"]["type"], "tool_use");
    assert_eq!(ev[1].1["delta"]["partial_json"], "{}");
    let id = ev[0].1["content_block"]["id"].as_str().unwrap();
    assert_eq!(sigs.get(id).as_deref(), Some("EMPTY_SIG"));
}

#[test]
fn finish_reasons() {
    let stop = |reason: &str| {
        let ev = run(&[last(json!([{ "text": "x" }]), reason)], &Sigs::default());
        ev.iter().find(|(n, _)| n == "message_delta").map(|(_, d)| d["delta"]["stop_reason"].clone())
    };
    assert_eq!(stop("MAX_TOKENS"), Some(json!("max_tokens")));
    // Gemini 4 hit its per-request limit before the end: a cut answer, not a whole one.
    assert_eq!(stop("CONTINUATION"), Some(json!("max_tokens")));
    assert_eq!(stop("SAFETY"), Some(json!("refusal")));
    assert_eq!(stop("PROHIBITED_CONTENT"), Some(json!("refusal")));
    for bad in ["MALFORMED_FUNCTION_CALL", "MISSING_THOUGHT_SIGNATURE"] {
        let ev = run(&[last(json!([{ "text": "x" }]), bad)], &Sigs::default());
        assert_eq!(names(&ev), ["content_block_start", "content_block_delta", "content_block_stop", "error"]);
        assert_eq!(ev[3].1["error"]["type"], "api_error");
        assert!(ev[3].1["error"]["message"].as_str().unwrap().contains(bad));
    }
}

#[test]
fn an_error_inside_the_stream() {
    let mut t = Translator::new("m");
    let ev = events(&t.chunk(&json!({ "error": { "code": 500, "message": "boom", "status": "INTERNAL" } }), &Sigs::default()));
    assert_eq!(ev[0].1, json!({ "type": "error", "error": { "type": "api_error", "message": "boom" } }));
    assert_eq!(t.finish(), "");
}

#[test]
fn a_whole_answer_as_one_message() {
    let sigs = Sigs::default();
    let resp = last(json!([
        { "text": "think", "thought": true },
        { "text": "Look" },
        { "functionCall": { "name": "Read", "args": { "path": "b" } }, "thoughtSignature": "S" }
    ]), "STOP");
    let m = to_message(&resp, "gemini-3.8-flash", &sigs);
    let id = m["content"][2]["id"].as_str().unwrap().to_string();
    assert!(m["id"].as_str().unwrap().starts_with("msg_"));
    let mut m2 = m.clone();
    m2["id"] = json!("x");
    assert_eq!(m2, json!({ "id": "x", "type": "message", "role": "assistant", "model": "gemini-3.8-flash", "content": [
        { "type": "thinking", "thinking": "think", "signature": "" },
        { "type": "text", "text": "Look" },
        { "type": "tool_use", "id": id, "name": "Read", "input": { "path": "b" } }
    ], "stop_reason": "tool_use", "stop_sequence": null,
        "usage": { "input_tokens": 6, "output_tokens": 7, "cache_creation_input_tokens": 0, "cache_read_input_tokens": 4 } }));
    assert_eq!(sigs.get(&id).as_deref(), Some("S"));
    let bad = to_message(&last(json!([]), "MALFORMED_FUNCTION_CALL"), "m", &sigs);
    assert_eq!(bad["type"], "error");
}
