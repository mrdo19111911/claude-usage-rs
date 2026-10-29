// Which Gemini models a key can use, and the one to take when the chosen default is not
// among them (an older key, a model retired since).

use crate::*;

// Google's list (GET /v1beta/models) → the names that can generate content, without
// "models/".
pub(crate) fn parse_models(j: &Value) -> Vec<String> {
    let all = j["models"].as_array().map(Vec::as_slice).unwrap_or_default();
    all.iter()
        .filter(|m| m["supportedGenerationMethods"].as_array().is_some_and(|a| a.iter().any(|x| x == "generateContent")))
        .filter_map(|m| m["name"].as_str())
        .map(|n| n.trim_start_matches("models/").to_string())
        .collect()
}

// Not for a chat with tools: pictures, speech, embeddings and the like.
fn chat(name: &str) -> bool {
    const NOT: [&str; 10] = ["embedding", "image", "tts", "audio", "live", "vision", "aqa", "robotics", "computer-use", "learnlm"];
    name.starts_with("gemini-") && !NOT.iter().any(|n| name.contains(n))
}

// "gemini-3.6-flash" → 3.6
fn version(name: &str) -> f64 {
    let v: String = name.trim_start_matches("gemini-").chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
    v.trim_end_matches('.').parse().unwrap_or(0.0)
}

fn stable(name: &str) -> bool {
    !name.contains("preview") && !name.contains("exp")
}

// `chosen` when the key has it; else the newest stable flash model of the kind (the
// fast one: "flash-lite"), a preview only when there is no stable one.
pub(crate) fn best(all: &[String], chosen: &str, fast: bool) -> String {
    if all.iter().any(|m| m == chosen) {
        return chosen.to_string();
    }
    let flash = |m: &&String| m.contains("flash") && !m.contains("lite");
    let lite = |m: &&String| m.contains("flash-lite");
    let usable: Vec<&String> = all.iter().filter(|m| chat(m)).collect();
    let pick = |f: &dyn Fn(&&String) -> bool| {
        usable.iter().copied().filter(|m| f(m)).max_by(|a, b| (stable(a), version(a)).partial_cmp(&(stable(b), version(b))).unwrap()).cloned()
    };
    let found = if fast { pick(&lite).or_else(|| pick(&flash)) } else { pick(&flash) };
    found.unwrap_or_else(|| chosen.to_string())
}
