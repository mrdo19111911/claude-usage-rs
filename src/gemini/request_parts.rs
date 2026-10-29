// Anthropic `messages` → Gemini `contents`: roles, blocks, the function name of each
// tool_result (from the tool_use with its id), and thought signatures on the calls.

use crate::*;
use crate::gemini::Sigs;
use super::order::{flatten, Item};
use std::collections::HashMap;

// Google's stand-in when the real signature is lost ("last resort", it lowers quality).
// Sent as is, like agcp and Google's docs; LiteLLM base64-encodes it first.
pub(super) const SKIP: &str = "skip_thought_signature_validator";

// `stand_in`: put SKIP on a model turn's first call when no signature is known (Gemini
// 3 refuses such a call; older models need none).
pub(super) fn contents(msgs: &[Value], sigs: &Sigs, stand_in: bool) -> Vec<Value> {
    // tool_use id → function name and the call's place among all calls so far (LiteLLM
    // and CCR look back the same way).
    let mut names: HashMap<String, (String, usize)> = HashMap::new();
    let mut out: Vec<(&str, Vec<Item>)> = vec![];
    for m in msgs {
        // A `system` message mid-conversation (Claude Code sends one to unknown models) is user text.
        let role = if m["role"] == "assistant" { "model" } else { "user" };
        let merging = out.last().is_some_and(|(r, _)| *r == role);
        // Only the first call of each model turn carries the signature (parallel calls).
        let mut first_done = merging && out.last().unwrap().1.iter().any(Item::is_call);
        let mut think_sig: Option<String> = None;
        let mut parts = vec![];
        for b in blocks(&m["content"]) {
            match b["type"].as_str().unwrap_or("") {
                "text" => parts.extend(text(b["text"].as_str().unwrap_or("")).map(Item::Part)),
                "image" | "document" => parts.extend(media(&b["source"]).map(Item::Part)),
                // The thought text is not sent back (Google: thoughts are not input); its
                // signature can stand for the call's own (CCR does this).
                "thinking" => think_sig = b["signature"].as_str().filter(|s| !s.is_empty()).map(String::from),
                "tool_use" => {
                    let id = b["id"].as_str().unwrap_or("");
                    let name = b["name"].as_str().unwrap_or("");
                    names.insert(id.to_string(), (name.to_string(), names.len()));
                    let args = if b["input"].is_object() { b["input"].clone() } else { json!({}) };
                    // No `id` sent: Gemini's own ids are not Claude Code's. The results
                    // go in call order instead (request_order.rs).
                    let mut p = json!({ "functionCall": { "name": name, "args": args } });
                    let fallback = || think_sig.clone().or_else(|| stand_in.then(|| SKIP.to_string()));
                    let sig = sigs.get(id).or_else(|| if first_done { None } else { fallback() });
                    if let Some(s) = sig {
                        p["thoughtSignature"] = json!(s);
                    }
                    first_done = true;
                    parts.push(Item::Part(p));
                }
                "tool_result" => parts.extend(result(&b, &names)),
                // redacted_thinking, server tool blocks: nothing Gemini can use.
                _ => {}
            }
        }
        match out.last_mut() {
            Some((r, p)) if *r == role => p.extend(parts),
            _ if !parts.is_empty() => out.push((role, parts)),
            _ => {}
        }
    }
    out.into_iter()
        .map(|(role, items)| (role, flatten(items)))
        .filter(|(_, p)| !p.is_empty())
        .map(|(role, parts)| json!({ "role": role, "parts": parts }))
        .collect()
}

// A message's content as blocks: a plain string is one text block.
fn blocks(c: &Value) -> Vec<Value> {
    match c {
        Value::String(s) => vec![json!({ "type": "text", "text": s })],
        Value::Array(a) => a.clone(),
        _ => vec![],
    }
}

// Gemini refuses an empty text part, and Anthropic's empty blocks carry nothing.
fn text(t: &str) -> Option<Value> {
    (!t.is_empty()).then(|| json!({ "text": t }))
}

// An image or document source: base64 → inlineData, url → fileData, text → text.
fn media(src: &Value) -> Option<Value> {
    let mime = src["media_type"].as_str();
    match src["type"].as_str()? {
        "base64" => Some(json!({ "inlineData": { "mimeType": mime?, "data": src["data"].as_str()? } })),
        "url" => {
            let mut f = json!({ "fileUri": src["url"].as_str()? });
            if let Some(m) = mime {
                f["mimeType"] = json!(m);
            }
            Some(json!({ "fileData": f }))
        }
        "text" => Some(json!({ "text": src["data"].as_str()? })),
        _ => None,
    }
}

// A tool_result → functionResponse {name, response: {result|error}}; its images follow
// as their own parts, and the two move together to their call's place. Without its call
// (cut from the history) it can only be text, left where it is.
fn result(b: &Value, names: &HashMap<String, (String, usize)>) -> Vec<Item> {
    let id = b["tool_use_id"].as_str().unwrap_or("");
    let mut texts = vec![];
    let mut extra = vec![];
    for c in blocks(&b["content"]) {
        match c["type"].as_str() {
            Some("text") => texts.push(c["text"].as_str().unwrap_or("").to_string()),
            Some("image" | "document") => extra.extend(media(&c["source"])),
            _ => {}
        }
    }
    let body = texts.join("\n");
    match names.get(id) {
        Some((name, place)) => {
            let key = if b["is_error"] == true { "error" } else { "result" };
            let resp = json!({ "functionResponse": { "name": name, "response": { key: body } } });
            vec![Item::Result(*place, [vec![resp], extra].concat())]
        }
        None => {
            let t = json!({ "text": format!("Tool result ({id}): {body}") });
            [vec![t], extra].concat().into_iter().map(Item::Part).collect()
        }
    }
}
