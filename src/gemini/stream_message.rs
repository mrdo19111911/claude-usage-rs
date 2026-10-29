// Stop reasons, ids, and a whole answer (`stream: false`) as one Anthropic message,
// built by folding the same events the stream sends, so both paths agree.

use crate::*;
use super::{Sigs, Translator};

// Gemini's finishReason → Anthropic's stop_reason, or the message of an `error` event.
pub(super) fn stop_reason(finish: Option<&str>, called: bool) -> Result<&'static str, String> {
    let done = if called { "tool_use" } else { "end_turn" };
    match finish {
        // No finishReason: the stream was cut, and a cut answer must not look whole.
        None => Err("Gemini's answer ended before it was finished".into()),
        // CONTINUATION: Gemini 4 hit its per-request limit before the end. Cut, as
        // OpenRouter says it; not resumed with continuationToken (spec, "The proxy").
        Some("MAX_TOKENS" | "CONTINUATION") => Ok("max_tokens"),
        Some(
            "SAFETY" | "RECITATION" | "BLOCKLIST" | "PROHIBITED_CONTENT" | "SPII" | "LANGUAGE" | "IMAGE_SAFETY"
            | "IMAGE_PROHIBITED_CONTENT" | "IMAGE_RECITATION",
        ) => Ok("refusal"),
        // Claude Code retries an error; these mean the call or the request was wrong.
        Some(
            r @ ("MALFORMED_FUNCTION_CALL" | "MISSING_THOUGHT_SIGNATURE" | "UNEXPECTED_TOOL_CALL" | "TOO_MANY_TOOL_CALLS"
            | "MALFORMED_RESPONSE"),
        ) => Err(format!("Gemini stopped with {r}")),
        Some(_) => Ok(done),
    }
}

// `prefix` and 24 random hex digits, the shape of Anthropic's ids (toolu_…, msg_…).
pub(super) fn hex_id(prefix: &str) -> String {
    let mut b = [0u8; 12];
    getrandom::getrandom(&mut b).expect("system random numbers");
    prefix.to_string() + &b.iter().map(|x| format!("{x:02x}")).collect::<String>()
}

// A whole GenerateContent answer (`stream: false`) → one Anthropic message, or the
// error JSON (`type: "error"`) when Gemini stopped on a bad call.
pub(crate) fn to_message(resp: &Value, model: &str, sigs: &Sigs) -> Value {
    let mut t = Translator::new(model);
    let mut ev = t.chunk_events(resp, sigs);
    ev.extend(t.finish_events());
    let mut content: Vec<Value> = vec![];
    let mut inputs: Vec<String> = vec![];
    let mut msg = json!({ "id": t.id, "type": "message", "role": "assistant", "model": model, "content": [],
        "stop_reason": null, "stop_sequence": null, "usage": {} });
    for e in ev {
        let i = e["index"].as_u64().unwrap_or(0) as usize;
        match e["type"].as_str().unwrap_or("") {
            "error" => return e,
            "content_block_start" => {
                let mut b = e["content_block"].clone();
                if b["type"] == "thinking" {
                    b["signature"] = json!("");
                }
                content.push(b);
                inputs.push(String::new());
            }
            "content_block_delta" => {
                let d = &e["delta"];
                let add = |b: &mut Value, k: &str, s: &Value| b[k] = json!(b[k].as_str().unwrap_or("").to_string() + s.as_str().unwrap_or(""));
                match d["type"].as_str().unwrap_or("") {
                    "text_delta" => add(&mut content[i], "text", &d["text"]),
                    "thinking_delta" => add(&mut content[i], "thinking", &d["thinking"]),
                    "signature_delta" => content[i]["signature"] = d["signature"].clone(),
                    "input_json_delta" => inputs[i] += d["partial_json"].as_str().unwrap_or(""),
                    _ => {}
                }
            }
            "message_delta" => {
                msg["stop_reason"] = e["delta"]["stop_reason"].clone();
                msg["usage"] = e["usage"].clone();
            }
            _ => {}
        }
    }
    for (b, input) in content.iter_mut().zip(inputs) {
        if b["type"] == "tool_use" {
            b["input"] = serde_json::from_str(&input).unwrap_or(json!({}));
        }
    }
    msg["content"] = Value::Array(content);
    msg
}
