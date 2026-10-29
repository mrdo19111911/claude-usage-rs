// Which Gemini models a key has, and which to use when the chosen one is missing.

use super::models::{best, parse_models, text_models};
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
    assert_eq!(best(&all, MODEL), MODEL);
    assert_eq!(best(&all, FAST_MODEL), FAST_MODEL);
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
    // Flash Lite first for both: 500 requests a day on a free key, Flash 20.
    assert_eq!(best(&all, MODEL), "gemini-3.1-flash-lite");
    assert_eq!(best(&all, FAST_MODEL), "gemini-3.1-flash-lite");
    // No lite model: Flash will do.
    assert_eq!(best(&names(&["gemini-3.6-flash"]), FAST_MODEL), "gemini-3.6-flash");
    assert_eq!(best(&names(&["gemini-3.6-flash"]), MODEL), "gemini-3.6-flash");
    // Nothing usable listed: keep the chosen name.
    assert_eq!(best(&[], MODEL), MODEL);
}

#[test]
fn the_defaults_are_the_two_lite_models() {
    assert_eq!((MODEL, FAST_MODEL), ("gemini-3.5-flash-lite", "gemini-3.1-flash-lite"));
}

#[test]
fn the_list_has_only_models_that_write_text() {
    let all = names(&[
        "gemini-3.5-flash-lite", "gemini-2.5-flash-preview-tts", "gemma-4-31b-it", "gemini-3.1-flash-image",
        "gemini-nano-banana-2.1", "gemini-omni-1.1-flash", "gemini-3.5-transcribe", "lyria-3.5",
        "gemini-robotics-er-2-preview", "deep-research-preview-04-2026", "antigravity-preview-latest", "gemini-3.8-flash",
    ]);
    assert_eq!(text_models(&all), names(&["gemini-3.5-flash-lite", "gemini-3.8-flash"]));
}
