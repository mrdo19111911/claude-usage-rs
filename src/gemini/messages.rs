// POST /v1/messages: the Anthropic request → Gemini, and Gemini's answer back, streamed
// as it comes (stream: true) or as one message.

use crate::*;
use super::proxy::reply;
use super::sse::Sse;
use super::stream::{self, Translator};
use super::{errors, relay, request, upstream, Config};

// Silence from Google longer than this: a ping to Claude Code (its idle watchdog).
const PING_AFTER: Duration = Duration::from_secs(10);

// A model name goes into Google's URL: letters, digits, '.', '-', '_' only.
fn plain(model: &str) -> bool {
    !model.is_empty() && model.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

// An error answer: the status, the Anthropic error body, `retry-after` seconds.
type Refusal = (u16, Value, Option<u64>);

// Google busy (503) or the model's count used up (429): the other model of the two
// answers instead, once (docs/specs/gemini.md, "The proxy"). The answer and its model.
fn call(app: &App, cfg: &Config, model: &str, body: &Value, stream: bool) -> Result<(ureq::Response, String), Refusal> {
    let other = if model == cfg.model { &cfg.fast_model } else if model == cfg.fast_model { &cfg.model } else { model };
    match call_one(app, cfg, model, body, stream) {
        Err((429 | 529, ..)) if other != model => call_one(app, cfg, other, body, stream).map(|r| (r, other.to_string())),
        r => r.map(|r| (r, model.to_string())),
    }
}

// Google's answer once it says 200. A 400 about a tool schema: once more with the older
// `parameters` form of the schemas (docs/SO_SANH_GEMINI_PROXY.md, Chọn 2).
fn call_one(app: &App, cfg: &Config, model: &str, body: &Value, stream: bool) -> Result<ureq::Response, Refusal> {
    let has_tools = body["tools"].as_array().is_some_and(|t| !t.is_empty());
    for legacy in [false, true] {
        let g = request::to_gemini(body, model, &app.gemini_sigs, legacy)
            .map_err(|e| (400, errors::error_json("invalid_request_error", &e), None))?;
        match upstream::generate(&cfg.key, model, stream, &g) {
            Ok(r) => return Ok(r),
            Err(ureq::Error::Status(code, r)) => {
                let text = r.into_string().unwrap_or_default();
                if !legacy && has_tools && errors::schema_refused(code, &text) {
                    continue;
                }
                return Err(errors::to_anthropic(code, &text));
            }
            Err(e) => {
                let msg = format!("Could not reach Gemini: {e}");
                return Err((502, errors::error_json("api_error", &msg), None));
            }
        }
    }
    unreachable!("the second try returns")
}

pub(crate) fn messages(app: &Arc<App>, cfg: &Config, rq: tiny_http::Request, body: &Value) {
    let model = request::pick_model(body["model"].as_str().unwrap_or(""), &cfg.model, &cfg.fast_model);
    if !plain(&model) {
        let msg = format!("Not a Gemini model name: {model}");
        return reply(rq, 400, &errors::error_json("invalid_request_error", &msg), None);
    }
    let streaming = body["stream"].as_bool().unwrap_or(false);
    let (resp, model) = match call(app, cfg, &model, body, streaming) {
        Ok(r) => r,
        Err((code, err, retry)) => return reply(rq, code, &err, retry),
    };
    if !streaming {
        return match resp.into_json::<Value>() {
            Ok(v) => {
                // Gemini stopped on a bad function call: an Anthropic error, not a message.
                let m = stream::to_message(&v, &model, &app.gemini_sigs);
                reply(rq, if m["type"] == "error" { 500 } else { 200 }, &m, None)
            }
            Err(e) => reply(rq, 502, &errors::error_json("api_error", &format!("Gemini's answer could not be read: {e}")), None),
        };
    }
    let Ok(mut out) = Sse::open(rq) else { return };
    let mut tr = Translator::new(&model);
    if out.send(&tr.start()).is_err() {
        return;
    }
    let sigs = &app.gemini_sigs;
    let done = relay::relay(resp.into_reader(), PING_AFTER, |c| tr.chunk(c, sigs), |s| out.send(s));
    // Ended cleanly: close the message. Broke off: the error event was the end. Client
    // gone: nothing more to say.
    if let Ok(true) = done {
        let _ = out.send(&tr.finish());
    }
    if done.is_ok() {
        out.end();
    }
}
