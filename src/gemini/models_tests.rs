// Which Gemini models a key has, and which to use when the chosen one is missing.

use super::models::{best, parse_models};
use super::{FAST_MODEL, MODEL};
use serde_json::json;

fn names(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

#[test]
fn keeps_models_that_generate_content() {
    let j = json!({ "models": [
        { "name": "models/gemini-3.8-flash", "supportedGenerationMethods": ["generateContent", "countTokens"] },
        { "name": "models/gemini-embedding-001", "supportedGenerationMethods": ["embedContent"] },
        { "name": "models/gemini-3.5-flash-lite", "supportedGenerationMethods": ["generateContent"] },
        { "name": "models/odd" },
    ]});
    assert_eq!(parse_models(&j), names(&["gemini-3.8-flash", "gemini-3.5-flash-lite"]));
    assert!(parse_models(&json!({})).is_empty());
}

#[test]
fn keeps_the_chosen_model_when_the_key_has_it() {
    let all = names(&[FAST_MODEL, "gemini-3.6-flash", MODEL]);
    assert_eq!(best(&all, MODEL, false), MODEL);
    assert_eq!(best(&all, FAST_MODEL, true), FAST_MODEL);
}

#[test]
fn else_the_newest_stable_of_its_kind() {
    let all = names(&[
        "gemini-3.1-flash-lite",
        "gemini-3-flash-preview",
        "gemini-3.6-flash",
        "gemini-3.6-flash-image",
        "gemini-4-argon",
        "gemini-3.1-pro-preview",
        "gemini-2.5-flash-lite",
        "gemini-embedding-001",
    ]);
    assert_eq!(best(&all, MODEL, false), "gemini-3.6-flash");
    assert_eq!(best(&all, FAST_MODEL, true), "gemini-3.1-flash-lite");
    // No lite model: the main kind will do for the fast one.
    assert_eq!(best(&names(&["gemini-3.6-flash"]), FAST_MODEL, true), "gemini-3.6-flash");
    // Nothing usable listed: keep the chosen name.
    assert_eq!(best(&[], MODEL, false), MODEL);
}
