// The calls to Google's Gemini API. The key goes in the `x-goog-api-key` header, never in
// a URL (URLs end up in error messages and logs).

use crate::*;
use super::models;
use std::sync::OnceLock;

const GOOGLE: &str = "https://generativelanguage.googleapis.com/v1beta";

// Tests point the calls at a fake Google on this machine (messages_tests.rs).

fn base() -> String {
    GOOGLE.to_string()
}

// One agent for the answers, shared (it keeps connections to Google open): no overall
// timeout, since an answer can stream for minutes; 30 s to connect; 120 s of silence.
fn agent() -> &'static ureq::Agent {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        ureq::AgentBuilder::new().timeout_connect(Duration::from_secs(30)).timeout_read(Duration::from_secs(120)).build()
    })
}

// POST models/<model>:streamGenerateContent?alt=sse (stream) or :generateContent.
pub(crate) fn generate(key: &str, model: &str, stream: bool, body: &Value) -> Result<ureq::Response, ureq::Error> {
    let how = if stream { "streamGenerateContent?alt=sse" } else { "generateContent" };
    agent()
        .post(&format!("{}/models/{model}:{how}", base()))
        .set("x-goog-api-key", key)
        .set("Content-Type", "application/json")
        .send_string(&body.to_string())
}

// Google's own message out of an error body, else the body itself (short).
pub(crate) fn google_message(body: &str) -> String {
    let v: Value = serde_json::from_str(body).unwrap_or(Value::Null);
    match v["error"]["message"].as_str() {
        Some(m) => m.to_string(),
        None => body.chars().take(300).collect(),
    }
}

// The check when a key is saved: the models it can generate with (10 s at most), or
// Google's reason for refusing it.
pub(crate) fn list_models(key: &str) -> Result<Vec<String>, String> {
    let agent = ureq::AgentBuilder::new().timeout(Duration::from_secs(10)).build();
    let r = agent.get(&format!("{}/models?pageSize=1000", base())).set("x-goog-api-key", key).call();
    match r {
        Ok(r) => {
            let j: Value = r.into_json().map_err(|e| format!("Google's answer could not be read: {e}"))?;
            let all = models::parse_models(&j);
            if all.is_empty() {
                return Err("Google lists no Gemini model for this key".into());
            }
            Ok(all)
        }
        Err(ureq::Error::Status(code, r)) => {
            let msg = google_message(&r.into_string().unwrap_or_default());
            Err(format!("Google refused the key ({code}): {msg}"))
        }
        Err(e) => Err(format!("Could not reach Google: {e}")),
    }
}
