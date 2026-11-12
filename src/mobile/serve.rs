// The feed's own listener (docs/specs/mobile-feed.md): all interfaces, one read-only
// route, a bearer token, private and Tailscale addresses only. The dashboard's
// 127.0.0.1 listener and its checks are not involved.
use super::{feed, pair, peer};
use std::thread;

pub(crate) const PATH: &str = "/api/mobile/v1/state";
pub(crate) type StateFn = Box<dyn Fn() -> String + Send + 'static>;

/// Starts listening (port 0: any free one) and returns the port.
pub(crate) fn start(state: StateFn, port: u16, token: String) -> std::io::Result<u16> {
    let server = tiny_http::Server::http(("0.0.0.0", port)).map_err(std::io::Error::other)?;
    let port = server.server_addr().to_ip().map(|a| a.port()).unwrap_or(port);
    thread::spawn(move || {
        for rq in server.incoming_requests() {
            let (code, body) = answer(&rq, &state, &token);
            let resp = tiny_http::Response::from_string(body)
                .with_status_code(code)
                .with_header(tiny_http::Header::from_bytes("Content-Type", "application/json").unwrap())
                .with_header(tiny_http::Header::from_bytes("Cache-Control", "no-store").unwrap());
            let _ = rq.respond(resp);
        }
    });
    Ok(port)
}

fn answer(rq: &tiny_http::Request, state: &StateFn, token: &str) -> (u16, String) {
    let err = |c: u16, m: &str| (c, format!(r#"{{"error":"{m}"}}"#));
    if !rq.remote_addr().is_some_and(|a| peer::allowed(a.ip())) {
        return err(403, "not from this network");
    }
    if rq.url().split('?').next() != Some(PATH) {
        return err(404, "not found");
    }
    if rq.method() != &tiny_http::Method::Get {
        return err(405, "read only");
    }
    let sent = rq.headers().iter().find(|h| h.field.equiv("Authorization")).map(|h| h.value.as_str());
    if !sent.and_then(|v| v.strip_prefix("Bearer ")).is_some_and(|t| pair::same(t.trim(), token)) {
        return err(401, "pair this phone again");
    }
    let public: serde_json::Value = serde_json::from_str(&state()).unwrap_or_default();
    (200, feed::from_public(&public).to_string())
}
