// The dashboard's port. Asked for any (0): the port of the last run again, kept in `port`
// next to state.json, so a `cgemini` session open across a restart (an update, a sign-out)
// carries on; a free one only when that is taken (docs/specs/modules.md).

use crate::*;

type Listened = Result<tiny_http::Server, Box<dyn std::error::Error + Send + Sync>>;

// `keep`: use and remember the saved port. Not in the demo: it runs beside the real one
// with the same data folder, and must not take that port (cgemini would reach the demo).
pub(crate) fn listen(data_dir: &Path, port: u16, keep: bool) -> Listened {
    if port != 0 || !keep {
        return tiny_http::Server::http(("127.0.0.1", port));
    }
    let file = data_dir.join("port");
    let last = fs::read_to_string(&file).ok().and_then(|s| s.trim().parse::<u16>().ok()).filter(|&p| p != 0);
    let server = match last.map(|p| tiny_http::Server::http(("127.0.0.1", p))) {
        Some(Ok(s)) => s,
        _ => tiny_http::Server::http(("127.0.0.1", 0))?,
    };
    if let Some(a) = server.server_addr().to_ip() {
        let _ = fs::create_dir_all(data_dir);
        let _ = fs::write(&file, a.port().to_string());
    }
    Ok(server)
}
