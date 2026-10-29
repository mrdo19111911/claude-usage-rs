// The rest of a request: model, generation config, thinking, tools, tool_choice, tokens.

use super::*;

fn base() -> Value {
    json!({ "model": "gemini-3.8-flash", "max_tokens": 1000, "messages": [{ "role": "user", "content": "hi" }] })
}

fn with(extra: Value) -> Value {
    let mut r = base();
    for (k, v) in extra.as_object().unwrap() {
        r[k] = v.clone();
    }
    to_gemini(&r, r["model"].as_str().unwrap(), &Sigs::default(), false).unwrap()
}

#[test]
fn the_model_for_what_claude_code_asks() {
    assert_eq!(pick_model("gemini-2.5-pro", "M", "F"), "gemini-2.5-pro");
    assert_eq!(pick_model("claude-haiku-4-5-20251001", "M", "F"), "F");
    assert_eq!(pick_model("claude-sonnet-4-5", "M", "F"), "M");
    assert_eq!(pick_model("", "M", "F"), "M");
}

#[test]
fn generation_config_with_the_output_capped() {
    let g = with(json!({ "max_tokens": 100000, "temperature": 0.5, "top_p": 0.9, "top_k": 40, "stop_sequences": ["END"] }));
    assert_eq!(g["generationConfig"], json!({
        "maxOutputTokens": 65536, "temperature": 0.5, "topP": 0.9, "topK": 40, "stopSequences": ["END"]
    }));
    assert_eq!(with(json!({}))["generationConfig"], json!({ "maxOutputTokens": 1000 }));
}

#[test]
fn thinking_asks_for_thoughts_and_effort_sets_the_level() {
    let on = with(json!({ "thinking": { "type": "enabled", "budget_tokens": 8000 } }));
    assert_eq!(on["generationConfig"]["thinkingConfig"], json!({ "includeThoughts": true }));
    let effort = with(json!({ "thinking": { "type": "adaptive" }, "output_config": { "effort": "max" } }));
    assert_eq!(effort["generationConfig"]["thinkingConfig"], json!({ "includeThoughts": true, "thinkingLevel": "high" }));
    let off = with(json!({ "thinking": { "type": "disabled" } }));
    assert!(off["generationConfig"].get("thinkingConfig").is_none());
    let old = with(json!({ "model": "gemini-2.5-flash", "thinking": { "type": "enabled", "budget_tokens": 50000 } }));
    assert_eq!(old["generationConfig"]["thinkingConfig"], json!({ "includeThoughts": true, "thinkingBudget": 24576 }));
}

#[test]
fn thinking_follows_the_model_called_not_the_name_asked() {
    let mut r = base();
    r["model"] = json!("claude-sonnet-4-5");
    r["thinking"] = json!({ "type": "enabled", "budget_tokens": 50000 });
    r["output_config"] = json!({ "effort": "high" });
    let pro = to_gemini(&r, "gemini-2.5-pro", &Sigs::default(), false).unwrap();
    assert_eq!(pro["generationConfig"]["thinkingConfig"], json!({ "includeThoughts": true, "thinkingBudget": 32768 }));
    let new = to_gemini(&r, "gemini-3.8-flash", &Sigs::default(), false).unwrap();
    assert_eq!(new["generationConfig"]["thinkingConfig"], json!({ "includeThoughts": true, "thinkingLevel": "high" }));
}

fn tools() -> Value {
    json!([
        { "name": "Read", "description": "Read a file", "input_schema": {
            "$schema": "http://json-schema.org/draft-07/schema#", "type": "object",
            "properties": { "path": { "type": "string" } }, "required": ["path"], "additionalProperties": false
        }},
        { "type": "web_search_20250305", "name": "web_search", "max_uses": 5 }
    ])
}

#[test]
fn tools_become_function_declarations() {
    let g = with(json!({ "tools": tools() }));
    assert_eq!(g["tools"], json!([{ "functionDeclarations": [{
        "name": "Read", "description": "Read a file", "parametersJsonSchema": {
            "type": "object", "properties": { "path": { "type": "string" } }, "required": ["path"], "additionalProperties": false
        }
    }]}]));
    let mut r = base();
    r["tools"] = tools();
    let old = to_gemini(&r, "gemini-3.8-flash", &Sigs::default(), true).unwrap();
    assert_eq!(old["tools"][0]["functionDeclarations"][0]["parameters"], json!({
        "type": "object", "properties": { "path": { "type": "string" } }, "required": ["path"]
    }));
}

#[test]
fn tool_choice_maps_and_needs_tools() {
    let mode = |choice: Value| with(json!({ "tools": tools(), "tool_choice": choice }))["toolConfig"].clone();
    assert_eq!(mode(json!({ "type": "auto" })), json!({ "functionCallingConfig": { "mode": "AUTO" } }));
    assert_eq!(mode(json!({ "type": "any" })), json!({ "functionCallingConfig": { "mode": "ANY" } }));
    assert_eq!(mode(json!({ "type": "none" })), json!({ "functionCallingConfig": { "mode": "NONE" } }));
    assert_eq!(mode(json!({ "type": "tool", "name": "Read" })), json!({ "functionCallingConfig": { "mode": "ANY", "allowedFunctionNames": ["Read"] } }));
    let bare = with(json!({ "tools": [tools()[1].clone()], "tool_choice": { "type": "auto" } }));
    assert!(bare.get("tools").is_none() && bare.get("toolConfig").is_none());
}

#[test]
fn a_request_without_messages_is_refused() {
    assert!(to_gemini(&json!({ "max_tokens": 1 }), "gemini-3.8-flash", &Sigs::default(), false).is_err());
}

#[test]
fn tokens_are_counted_at_four_characters_each() {
    let r = json!({ "messages": [{ "role": "user", "content": "x".repeat(400) }] });
    let n = count_tokens(&r);
    assert!((100..=115).contains(&n), "{n}");
    let mut more = r.clone();
    more["system"] = json!("y".repeat(400));
    assert!(count_tokens(&more) >= n + 100);
}
