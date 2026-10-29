// Google's error answers → Anthropic's, which Claude Code knows how to retry or show.

use crate::*;

// Google's status and body → the status to send, the Anthropic error JSON and, for a
// 429, the seconds for `retry-after`.
pub(crate) fn to_anthropic(status: u16, body: &str) -> (u16, Value, Option<u64>) {
    let _ = (status, body);
    todo!()
}

// {"type":"error","error":{"type":kind,"message":msg}}
pub(crate) fn error_json(kind: &str, msg: &str) -> Value {
    json!({ "type": "error", "error": { "type": kind, "message": msg } })
}
