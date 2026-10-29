// Gemini's answer → Anthropic's: SSE events as chunks arrive, or one message. Ported
// from agcp's cloudcode/sse.rs: one block counter, a block closes when the part kind
// changes, each functionCall is a whole tool_use block (Gemini never splits one).

use crate::*;
use super::Sigs;

#[path = "stream_message.rs"]
mod message;
pub(crate) use message::to_message;
use message::{hex_id, stop_reason};

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Text,
    Thinking,
}

pub(crate) struct Translator {
    pub(crate) model: String,
    id: String,
    // The index the next block gets, and the text or thinking block now open
    // (a tool_use block closes as soon as it is sent).
    index: usize,
    open: Option<Kind>,
    // The open thinking block's signature, sent as signature_delta before it closes.
    thought_sig: String,
    // A signature on a part that is not a call (Google: in streaming, often an empty
    // text part); the next call without its own gets it.
    loose_sig: String,
    called: bool,
    finish: Option<String>,
    // promptTokenCount, cachedContentTokenCount, candidates + thoughts (both billed as output).
    usage: (u64, u64, u64),
    // An error went out, or the end did: nothing more is sent.
    done: bool,
}

impl Translator {
    pub(crate) fn new(model: &str) -> Self {
        Translator {
            model: model.into(), id: hex_id("msg_"), index: 0, open: None, thought_sig: String::new(),
            loose_sig: String::new(), called: false, finish: None, usage: (0, 0, 0), done: false,
        }
    }
    // message_start and ping, once Google has answered 200.
    pub(crate) fn start(&mut self) -> String {
        sse(&self.start_events())
    }
    // One `data:` chunk of Google's stream, parsed: the SSE text to send on.
    pub(crate) fn chunk(&mut self, c: &Value, sigs: &Sigs) -> String {
        sse(&self.chunk_events(c, sigs))
    }
    // The open block closed, message_delta (stop reason, usage), message_stop.
    pub(crate) fn finish(&mut self) -> String {
        sse(&self.finish_events())
    }

    fn start_events(&self) -> Vec<Value> {
        let usage = json!({ "input_tokens": 0, "output_tokens": 0, "cache_creation_input_tokens": 0, "cache_read_input_tokens": 0 });
        let msg = json!({ "id": self.id, "type": "message", "role": "assistant", "model": self.model, "content": [],
            "stop_reason": null, "stop_sequence": null, "usage": usage });
        vec![json!({ "type": "message_start", "message": msg }), json!({ "type": "ping" })]
    }

    fn chunk_events(&mut self, c: &Value, sigs: &Sigs) -> Vec<Value> {
        if self.done {
            return vec![];
        }
        if let Some(code) = c.get("error").map(|e| e["code"].as_u64().unwrap_or(500)) {
            self.done = true;
            return vec![super::errors::to_anthropic(code as u16, &c.to_string()).1];
        }
        let u = &c["usageMetadata"];
        if u.is_object() {
            let n = |k: &str| u[k].as_u64().unwrap_or(0);
            self.usage = (n("promptTokenCount"), n("cachedContentTokenCount"), n("candidatesTokenCount") + n("thoughtsTokenCount"));
        }
        if c["promptFeedback"]["blockReason"].is_string() {
            self.finish = Some("BLOCKLIST".into());
        }
        let mut ev = vec![];
        let cand = &c["candidates"][0];
        for p in cand["content"]["parts"].as_array().into_iter().flatten() {
            self.part(p, sigs, &mut ev);
        }
        if let Some(r) = cand["finishReason"].as_str() {
            self.finish = Some(r.to_string());
        }
        ev
    }

    fn part(&mut self, p: &Value, sigs: &Sigs, ev: &mut Vec<Value>) {
        let sig = p["thoughtSignature"].as_str().unwrap_or("");
        if let Some(fc) = p.get("functionCall") {
            self.close(ev);
            let id = hex_id("toolu_");
            let sig = if sig.is_empty() { std::mem::take(&mut self.loose_sig) } else { sig.to_string() };
            sigs.put(&id, &sig);
            let args = if fc["args"].is_object() { fc["args"].to_string() } else { "{}".into() };
            let i = self.index;
            ev.push(json!({ "type": "content_block_start", "index": i, "content_block": { "type": "tool_use", "id": id, "name": fc["name"], "input": {} } }));
            ev.push(json!({ "type": "content_block_delta", "index": i, "delta": { "type": "input_json_delta", "partial_json": args } }));
            ev.push(json!({ "type": "content_block_stop", "index": i }));
            self.index += 1;
            self.called = true;
            return;
        }
        let Some(text) = p["text"].as_str() else { return };
        let thought = p["thought"] == true;
        if !sig.is_empty() {
            if thought && (self.open == Some(Kind::Thinking) || !text.is_empty()) {
                self.thought_sig = sig.to_string();
            } else {
                self.loose_sig = sig.to_string();
            }
        }
        if text.is_empty() {
            return;
        }
        let (kind, delta) = if thought {
            (Kind::Thinking, json!({ "type": "thinking_delta", "thinking": text }))
        } else {
            (Kind::Text, json!({ "type": "text_delta", "text": text }))
        };
        if self.open != Some(kind) {
            self.close(ev);
            let block = if thought { json!({ "type": "thinking", "thinking": "" }) } else { json!({ "type": "text", "text": "" }) };
            ev.push(json!({ "type": "content_block_start", "index": self.index, "content_block": block }));
            self.open = Some(kind);
        }
        ev.push(json!({ "type": "content_block_delta", "index": self.index, "delta": delta }));
    }

    fn close(&mut self, ev: &mut Vec<Value>) {
        let Some(kind) = self.open.take() else { return };
        if kind == Kind::Thinking && !self.thought_sig.is_empty() {
            let s = std::mem::take(&mut self.thought_sig);
            ev.push(json!({ "type": "content_block_delta", "index": self.index, "delta": { "type": "signature_delta", "signature": s } }));
        }
        ev.push(json!({ "type": "content_block_stop", "index": self.index }));
        self.index += 1;
    }

    fn finish_events(&mut self) -> Vec<Value> {
        if self.done {
            return vec![];
        }
        self.done = true;
        let mut ev = vec![];
        self.close(&mut ev);
        match stop_reason(self.finish.as_deref(), self.called) {
            Ok(reason) => {
                let (prompt, cached, out) = self.usage;
                let usage = json!({ "input_tokens": prompt.saturating_sub(cached), "output_tokens": out,
                    "cache_creation_input_tokens": 0, "cache_read_input_tokens": cached });
                ev.push(json!({ "type": "message_delta", "delta": { "stop_reason": reason, "stop_sequence": null }, "usage": usage }));
                ev.push(json!({ "type": "message_stop" }));
            }
            Err(msg) => ev.push(super::errors::error_json("api_error", &msg)),
        }
        ev
    }
}

// Events → SSE text; the event name is each one's `type`.
fn sse(events: &[Value]) -> String {
    events.iter().map(|e| format!("event: {}\ndata: {}\n\n", e["type"].as_str().unwrap_or("error"), e)).collect()
}

// Sent while Google says nothing for 10 s, so Claude Code does not give up.
pub(crate) fn ping() -> String {
    "event: ping\ndata: {\"type\":\"ping\"}\n\n".into()
}
