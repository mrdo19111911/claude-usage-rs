// An Anthropic Messages request → a Gemini GenerateContent body
// (docs/SO_SANH_GEMINI_PROXY.md, Chọn 2–4). The messages themselves: request_parts.rs.

use crate::*;
use super::Sigs;

#[path = "request_parts.rs"]
mod parts;
// Tool results in the order of their calls.
#[path = "request_order.rs"]
mod order;

// Gemini's output ceiling; more is refused rather than cut (agcp caps the same way).
const MAX_OUT: u64 = 65536;

// The Gemini model for the model Claude Code asks for: a Gemini name as is; a Claude
// name with "haiku" → `fast`; any other → `main`.
pub(crate) fn pick_model(asked: &str, main: &str, fast: &str) -> String {
    let a = asked.trim().trim_start_matches("models/");
    if a.to_ascii_lowercase().starts_with("gemini") {
        a.to_string()
    } else if a.to_ascii_lowercase().contains("haiku") {
        fast.to_string()
    } else {
        main.to_string()
    }
}

// Gemini 1.x/2.x: no thought signatures, and thinking set by a token budget.
fn older(model: &str) -> bool {
    let m = model.to_ascii_lowercase();
    m.starts_with("gemini-1") || m.starts_with("gemini-2")
}

// `model`: the Gemini model this body goes to, not the name Claude Code sent (a Claude
// name, or the other model on a retry): thinking and signatures follow it (spec, "The
// proxy"). `legacy`: tool schemas in the older `parameters` form (the retry after Google
// refuses a schema), else `parametersJsonSchema`.
pub(crate) fn to_gemini(req: &Value, model: &str, sigs: &Sigs, legacy: bool) -> Result<Value, String> {
    let msgs = req["messages"].as_array().ok_or("`messages` is missing")?;
    let old = older(model);
    let contents = parts::contents(msgs, sigs, !old);
    if contents.is_empty() {
        return Err("`messages` has nothing to send".into());
    }
    let mut body = json!({ "contents": contents });
    if let Some(s) = system(&req["system"]) {
        body["systemInstruction"] = s;
    }
    body["generationConfig"] = generation(req, model);
    if let Some(decls) = declarations(&req["tools"], legacy) {
        body["tools"] = json!([{ "functionDeclarations": decls }]);
        // Only with tools: a toolConfig alone is refused.
        if let Some(tc) = tool_config(&req["tool_choice"]) {
            body["toolConfig"] = tc;
        }
    }
    Ok(body)
}

// `system`, a string or text blocks (cache_control dropped) → systemInstruction.
fn system(s: &Value) -> Option<Value> {
    let parts: Vec<Value> = match s {
        Value::String(t) => vec![t.clone()],
        Value::Array(a) => a.iter().filter_map(|b| b["text"].as_str().map(String::from)).collect(),
        _ => vec![],
    }
    .into_iter()
    .filter(|t| !t.is_empty())
    .map(|t| json!({ "text": t }))
    .collect();
    (!parts.is_empty()).then(|| json!({ "parts": parts }))
}

fn generation(req: &Value, model: &str) -> Value {
    let max = req["max_tokens"].as_u64().unwrap_or(MAX_OUT).min(MAX_OUT);
    let mut g = json!({ "maxOutputTokens": max });
    for (from, to) in [("temperature", "temperature"), ("top_p", "topP"), ("top_k", "topK"), ("stop_sequences", "stopSequences")] {
        let v = &req[from];
        if !v.is_null() && v.as_array().is_none_or(|a| !a.is_empty()) {
            g[to] = v.clone();
        }
    }
    if let Some(t) = thinking(req, model) {
        g["thinkingConfig"] = t;
    }
    g
}

// Thinking on (enabled or adaptive) → thoughts included. Gemini 3 takes a level, from
// Claude Code's effort; Gemini 2.x a budget within its range (CCR's numbers). Off or
// absent → the model's own default.
fn thinking(req: &Value, model: &str) -> Option<Value> {
    if !matches!(req["thinking"]["type"].as_str(), Some("enabled" | "adaptive")) {
        return None;
    }
    let mut t = json!({ "includeThoughts": true });
    if older(model) {
        if let Some(b) = req["thinking"]["budget_tokens"].as_u64() {
            let (lo, hi) = if model.contains("pro") { (128, 32768) } else { (0, 24576) };
            t["thinkingBudget"] = json!(b.clamp(lo, hi));
        }
    } else if let Some(e) = req["output_config"]["effort"].as_str() {
        let level = match e {
            "low" => "low",
            "medium" => "medium",
            _ => "high",
        };
        t["thinkingLevel"] = json!(level);
    }
    Some(t)
}

// Tools with an input_schema → function declarations. Server tools (web_search,
// code execution...) have none and Gemini could not run them: dropped.
fn declarations(tools: &Value, legacy: bool) -> Option<Vec<Value>> {
    let decls: Vec<Value> = tools
        .as_array()?
        .iter()
        .filter(|t| t["input_schema"].is_object())
        .map(|t| {
            let mut d = json!({ "name": t["name"] });
            if let Some(desc) = t["description"].as_str() {
                d["description"] = json!(desc);
            }
            let s = &t["input_schema"];
            if legacy {
                d["parameters"] = super::schema::legacy(s);
            } else {
                d["parametersJsonSchema"] = super::schema::clean(s);
            }
            d
        })
        .collect();
    (!decls.is_empty()).then_some(decls)
}

// tool_choice → functionCallingConfig (agcp to_google.rs, Google's API reference).
fn tool_config(choice: &Value) -> Option<Value> {
    let fc = match choice["type"].as_str()? {
        "auto" => json!({ "mode": "AUTO" }),
        "any" => json!({ "mode": "ANY" }),
        "none" => json!({ "mode": "NONE" }),
        "tool" => json!({ "mode": "ANY", "allowedFunctionNames": [choice["name"]] }),
        _ => return None,
    };
    Some(json!({ "functionCallingConfig": fc }))
}

// /v1/messages/count_tokens: an estimate, characters / 4.
pub(crate) fn count_tokens(req: &Value) -> u64 {
    let len = |v: &Value| match v {
        Value::Null => 0,
        Value::String(s) => s.chars().count(),
        v => v.to_string().chars().count(),
    };
    let n = len(&req["system"]) + len(&req["messages"]) + len(&req["tools"]);
    (n as u64 / 4).max(1)
}
