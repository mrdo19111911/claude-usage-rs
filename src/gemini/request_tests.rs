// Messages: system, roles, text, images, tool calls and results, thought signatures.

use super::*;

const SKIP: &str = "skip_thought_signature_validator";

fn tr(req: Value) -> Value {
    to_gemini(&req, &Sigs::default(), false).unwrap()
}

fn msgs(m: Value) -> Value {
    tr(json!({ "model": "gemini-3.8-flash", "max_tokens": 100, "messages": m }))["contents"].clone()
}

#[test]
fn system_as_a_string_or_blocks() {
    let one = tr(json!({ "max_tokens": 1, "system": "Be brief", "messages": [{ "role": "user", "content": "hi" }] }));
    assert_eq!(one["systemInstruction"], json!({ "parts": [{ "text": "Be brief" }] }));
    let blocks = json!([{ "type": "text", "text": "A" }, { "type": "text", "text": "B", "cache_control": { "type": "ephemeral" } }]);
    let two = tr(json!({ "max_tokens": 1, "system": blocks, "messages": [{ "role": "user", "content": "hi" }] }));
    assert_eq!(two["systemInstruction"], json!({ "parts": [{ "text": "A" }, { "text": "B" }] }));
    let none = tr(json!({ "max_tokens": 1, "messages": [{ "role": "user", "content": "hi" }] }));
    assert!(none.get("systemInstruction").is_none());
}

#[test]
fn roles_map_and_same_role_turns_merge() {
    let c = msgs(json!([
        { "role": "user", "content": "a" },
        { "role": "user", "content": [{ "type": "text", "text": "b", "cache_control": { "type": "ephemeral" } }] },
        { "role": "assistant", "content": "c" }
    ]));
    assert_eq!(c, json!([
        { "role": "user", "parts": [{ "text": "a" }, { "text": "b" }] },
        { "role": "model", "parts": [{ "text": "c" }] }
    ]));
}

#[test]
fn a_system_message_mid_conversation_is_user_text() {
    let c = msgs(json!([
        { "role": "user", "content": "a" },
        { "role": "system", "content": "rule" },
        { "role": "assistant", "content": "b" }
    ]));
    assert_eq!(c[0], json!({ "role": "user", "parts": [{ "text": "a" }, { "text": "rule" }] }));
    assert_eq!(c[1]["role"], "model");
}

#[test]
fn text_and_a_base64_image() {
    let c = msgs(json!([{ "role": "user", "content": [
        { "type": "text", "text": "what is it" },
        { "type": "image", "source": { "type": "base64", "media_type": "image/png", "data": "iVBOR" } }
    ]}]));
    assert_eq!(c[0]["parts"], json!([{ "text": "what is it" }, { "inlineData": { "mimeType": "image/png", "data": "iVBOR" } }]));
}

fn tool_turns(result: Value) -> Value {
    msgs(json!([
        { "role": "user", "content": "read it" },
        { "role": "assistant", "content": [
            { "type": "text", "text": "Reading" },
            { "type": "tool_use", "id": "toolu_1", "name": "Read", "input": { "path": "a.txt" } }
        ]},
        { "role": "user", "content": [result] }
    ]))
}

#[test]
fn a_tool_call_and_its_result_with_the_name_from_the_call() {
    let c = tool_turns(json!({ "type": "tool_result", "tool_use_id": "toolu_1", "content": "hello" }));
    assert_eq!(c[1], json!({ "role": "model", "parts": [
        { "text": "Reading" },
        { "functionCall": { "name": "Read", "args": { "path": "a.txt" } }, "thoughtSignature": SKIP }
    ]}));
    assert_eq!(c[2], json!({ "role": "user", "parts": [
        { "functionResponse": { "name": "Read", "response": { "result": "hello" } } }
    ]}));
}

#[test]
fn a_tool_result_of_blocks_and_an_error() {
    let c = tool_turns(json!({ "type": "tool_result", "tool_use_id": "toolu_1", "is_error": true, "content": [
        { "type": "text", "text": "no" }, { "type": "text", "text": "such file" },
        { "type": "image", "source": { "type": "base64", "media_type": "image/jpeg", "data": "/9j" } }
    ]}));
    assert_eq!(c[2]["parts"], json!([
        { "functionResponse": { "name": "Read", "response": { "error": "no\nsuch file" } } },
        { "inlineData": { "mimeType": "image/jpeg", "data": "/9j" } }
    ]));
}

#[test]
fn a_result_without_its_call_becomes_text() {
    let c = msgs(json!([{ "role": "user", "content": [
        { "type": "tool_result", "tool_use_id": "toolu_gone", "content": "42" }
    ]}]));
    assert_eq!(c[0]["parts"], json!([{ "text": "Tool result (toolu_gone): 42" }]));
}

#[test]
fn the_signature_comes_from_the_cache_by_id() {
    let sigs = Sigs::default();
    sigs.put("toolu_1", "REAL_SIG");
    let req = json!({ "max_tokens": 1, "messages": [
        { "role": "user", "content": "go" },
        { "role": "assistant", "content": [{ "type": "tool_use", "id": "toolu_1", "name": "Bash", "input": {} }] },
        { "role": "user", "content": [{ "type": "tool_result", "tool_use_id": "toolu_1", "content": "ok" }] }
    ]});
    let c = &to_gemini(&req, &sigs, false).unwrap()["contents"];
    assert_eq!(c[1]["parts"][0]["thoughtSignature"], "REAL_SIG");
    assert!(c[1]["parts"][0]["functionCall"].get("id").is_none());
    assert!(c[2]["parts"][0]["functionResponse"].get("id").is_none());
}

#[test]
fn only_the_first_call_of_each_model_turn_gets_the_stand_in() {
    let call = |id: &str| json!({ "type": "tool_use", "id": id, "name": "Bash", "input": {} });
    let res = |id: &str| json!({ "type": "tool_result", "tool_use_id": id, "content": "ok" });
    let c = msgs(json!([
        { "role": "user", "content": "go" },
        { "role": "assistant", "content": [call("t1"), call("t2")] },
        { "role": "user", "content": [res("t1"), res("t2")] },
        { "role": "assistant", "content": [call("t3")] },
        { "role": "user", "content": [res("t3")] }
    ]));
    assert_eq!(c[1]["parts"][0]["thoughtSignature"], SKIP);
    assert!(c[1]["parts"][1].get("thoughtSignature").is_none());
    assert_eq!(c[3]["parts"][0]["thoughtSignature"], SKIP);
}

#[test]
fn a_thinking_block_lends_its_signature_and_is_not_sent() {
    let c = msgs(json!([
        { "role": "user", "content": "go" },
        { "role": "assistant", "content": [
            { "type": "thinking", "thinking": "hmm", "signature": "THINK_SIG" },
            { "type": "redacted_thinking", "data": "xyz" },
            { "type": "tool_use", "id": "toolu_9", "name": "Bash", "input": { "c": "ls" } }
        ]}
    ]));
    assert_eq!(c[1]["parts"], json!([{ "functionCall": { "name": "Bash", "args": { "c": "ls" } }, "thoughtSignature": "THINK_SIG" }]));
}

#[test]
fn turns_left_empty_are_dropped() {
    let c = msgs(json!([
        { "role": "user", "content": "a" },
        { "role": "assistant", "content": [{ "type": "thinking", "thinking": "x", "signature": "" }] },
        { "role": "user", "content": [{ "type": "text", "text": "" }, { "type": "text", "text": "b" }] }
    ]));
    assert_eq!(c, json!([{ "role": "user", "parts": [{ "text": "a" }, { "text": "b" }] }]));
}
