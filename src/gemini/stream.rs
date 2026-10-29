// Gemini's answer → Anthropic's: SSE events as chunks arrive, or one message.

use crate::*;
use super::Sigs;

pub(crate) struct Translator {
    // Filled in by the implementation: the model name, the next block index, the open
    // block, whether a tool was called, the usage so far.
    pub(crate) model: String,
}

impl Translator {
    pub(crate) fn new(model: &str) -> Self {
        Translator { model: model.into() }
    }
    // message_start and ping, once Google has answered 200.
    pub(crate) fn start(&mut self) -> String {
        todo!()
    }
    // One `data:` chunk of Google's stream, parsed: the SSE text to send on.
    pub(crate) fn chunk(&mut self, c: &Value, sigs: &Sigs) -> String {
        let _ = (c, sigs);
        todo!()
    }
    // The open block closed, message_delta (stop reason, usage), message_stop.
    pub(crate) fn finish(&mut self) -> String {
        todo!()
    }
}

// Sent while Google says nothing for 10 s, so Claude Code does not give up.
pub(crate) fn ping() -> String {
    "event: ping\ndata: {\"type\":\"ping\"}\n\n".into()
}

// A whole GenerateContent answer (`stream: false`) → one Anthropic message.
pub(crate) fn to_message(resp: &Value, model: &str, sigs: &Sigs) -> Value {
    let _ = (resp, model, sigs);
    todo!()
}
