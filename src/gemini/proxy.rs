// /v1/* for Claude Code (cgemini): this machine's own host names, then the token, then
// the route. Anthropic-shaped errors throughout, which Claude Code knows how to show.

use crate::*;
use super::{errors, messages, request};

#[derive(PartialEq, Debug)]
pub(crate) enum Route {
    Messages,
    CountTokens,
    Models,
    Ping,
    NotFound,
}

// By path, without the query (Claude Code adds ?beta=true).
pub(crate) fn route(method: &str, path: &str) -> Route {
    match (method, path) {
        ("POST", "/v1/messages") => Route::Messages,
        ("POST", "/v1/messages/count_tokens") => Route::CountTokens,
        ("GET", "/v1/models") => Route::Models,
        ("GET", "/v1/ping") => Route::Ping,
        _ => Route::NotFound,
    }
}

// Equal, in a time that does not tell how much of the start matched.
pub(crate) fn same(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |d, (x, y)| d | (x ^ y)) == 0
}

// `Authorization: Bearer <token>` (ANTHROPIC_AUTH_TOKEN), `x-api-key` (ANTHROPIC_API_KEY) or
// `x-cud-token` (ANTHROPIC_CUSTOM_HEADERS: Claude Code signed in to claude.ai sends its own
// login in Authorization instead of the token).
pub(crate) fn token_ok(auth: Option<&str>, x_api_key: Option<&str>, x_cud: Option<&str>, token: &str) -> bool {
    if token.is_empty() {
        return false;
    }
    let bearer = auth.and_then(|a| a.get(..7).filter(|s| s.eq_ignore_ascii_case("bearer ")).map(|_| &a[7..]));
    let is = |v: Option<&str>| v.is_some_and(|v| same(v.trim(), token));
    is(bearer) | is(x_api_key) | is(x_cud)
}

fn header<'a>(rq: &'a tiny_http::Request, name: &'static str) -> Option<&'a str> {
    rq.headers().iter().find(|h| h.field.equiv(name)).map(|h| h.value.as_str())
}

pub(crate) fn reply(rq: tiny_http::Request, code: u16, body: &Value, retry_after: Option<u64>) {
    let mut resp = tiny_http::Response::from_string(body.to_string())
        .with_status_code(code)
        .with_header(tiny_http::Header::from_bytes("Content-Type", "application/json").unwrap());
    if let Some(s) = retry_after {
        resp = resp.with_header(tiny_http::Header::from_bytes("retry-after", s.to_string()).unwrap());
    }
    let _ = rq.respond(resp);
}

pub(crate) fn serve(app: &Arc<App>, mut rq: tiny_http::Request) {
    // As server.rs: a page that points its own domain at 127.0.0.1 is not let in.
    let host = header(&rq, "Host").unwrap_or("");
    if host != format!("127.0.0.1:{}", app.port) && host != format!("localhost:{}", app.port) {
        return reply(rq, 403, &errors::error_json("permission_error", "forbidden host"), None);
    }
    let path = rq.url().split('?').next().unwrap_or("").to_string();
    let route = route(rq.method().as_str(), &path);
    // Only the app-is-running check of cgemini; it says nothing else.
    if route == Route::Ping {
        return reply(rq, 200, &json!({}), None);
    }
    let cfg = app.st.lock().unwrap().gemini.clone();
    let token = cfg.as_ref().map_or("", |c| c.token.as_str());
    if !token_ok(header(&rq, "Authorization"), header(&rq, "x-api-key"), header(&rq, "x-cud-token"), token) {
        let msg = if cfg.is_none() { "No Gemini key is saved: add one in Claude Usage." } else { "Wrong token: run cgemini again." };
        return reply(rq, 401, &errors::error_json("authentication_error", msg), None);
    }
    let Some(cfg) = cfg else { return };
    match route {
        Route::Models => return reply(rq, 200, &json!({ "data": [] }), None),
        Route::NotFound => return reply(rq, 404, &errors::error_json("not_found_error", &format!("No route {path}")), None),
        _ => {}
    }
    let mut text = String::new();
    let body: Option<Value> = rq.as_reader().read_to_string(&mut text).ok().and_then(|_| serde_json::from_str(&text).ok());
    let Some(body) = body.filter(Value::is_object) else {
        return reply(rq, 400, &errors::error_json("invalid_request_error", "The body is not a JSON object."), None);
    };
    match route {
        Route::CountTokens => reply(rq, 200, &json!({ "input_tokens": request::count_tokens(&body) }), None),
        _ => messages::messages(app, &cfg, rq, &body),
    }
}
