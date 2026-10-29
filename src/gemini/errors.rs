// Google's error answers → Anthropic's, which Claude Code knows how to retry or show.
// Google's body: {"error":{"code","message","status","details":[{"@type",...}]}}.

use crate::*;

// Google's status and body → the status to send, the Anthropic error JSON and, for a
// 429, the seconds for `retry-after`.
pub(crate) fn to_anthropic(status: u16, body: &str) -> (u16, Value, Option<u64>) {
    let err = serde_json::from_str::<Value>(body).ok().and_then(|v| v.get("error").cloned()).unwrap_or(Value::Null);
    let details = err["details"].as_array().cloned().unwrap_or_default();
    let mut msg = err["message"].as_str().unwrap_or(body.trim()).to_string();
    if msg.is_empty() {
        msg = format!("Gemini answered HTTP {status}");
    }
    // A wrong key comes back as 400 INVALID_ARGUMENT; Claude Code shows a 401 as a login problem.
    let bad_key = details.iter().any(|d| d["reason"] == "API_KEY_INVALID");
    let (to, kind) = match status {
        400 if bad_key => (401, "authentication_error"),
        400 => (400, "invalid_request_error"),
        401 => (401, "authentication_error"),
        403 => (403, "permission_error"),
        404 => (404, "not_found_error"),
        413 => (413, "request_too_large"),
        429 => (429, "rate_limit_error"),
        // Claude Code waits and retries on 529, as on Anthropic's own overload.
        503 => (529, "overloaded_error"),
        _ => (status, "api_error"),
    };
    let retry = if status == 429 { retry_secs(&details) } else { None };
    (to, error_json(kind, &msg), retry)
}

// RetryInfo.retryDelay ("13s", "1.2s") rounded up to whole seconds, as retry-after wants.
fn retry_secs(details: &[Value]) -> Option<u64> {
    let d = details.iter().find(|d| d["@type"].as_str().is_some_and(|t| t.ends_with("RetryInfo")))?;
    let secs: f64 = d["retryDelay"].as_str()?.strip_suffix('s')?.parse().ok()?;
    Some(secs.max(0.0).ceil() as u64)
}

// {"type":"error","error":{"type":kind,"message":msg}}
pub(crate) fn error_json(kind: &str, msg: &str) -> Value {
    json!({ "type": "error", "error": { "type": kind, "message": msg } })
}

// A 400 about a tool schema: the proxy then sends the request again with `legacy`.
pub(crate) fn schema_refused(status: u16, body: &str) -> bool {
    let b = body.to_ascii_lowercase();
    status == 400 && ["schema", "function_declarations", "functiondeclarations", "parameters"].iter().any(|k| b.contains(k))
}
