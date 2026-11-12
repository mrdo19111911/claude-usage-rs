// Which pages stay in the dashboard window; the rest open in the browser
// (docs/specs/dashboard-window.md).

// True for the dashboard's own pages: http on 127.0.0.1 or localhost at `port`, nothing
// after the port but a path, a query or a fragment (no look-alike host).
pub(super) fn stays_inside(url: &str, port: u16) -> bool {
    ["127.0.0.1", "localhost"].iter().any(|h| {
        let rest = url.strip_prefix(&format!("http://{h}:{port}"));
        rest.is_some_and(|r| r.is_empty() || r.starts_with(['/', '?', '#']))
    })
}
