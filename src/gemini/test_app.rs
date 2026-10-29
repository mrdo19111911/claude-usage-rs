// Tests only: an App in a temporary folder, and the real server around it on a free port.

use crate::*;
use super::Config;

// A fake key: never valid at Google, and easy to look for in answers.
pub(crate) const KEY: &str = "AIzaFAKE-never-shown-K3Y-wxyz";
pub(crate) const TOKEN: &str = "cud-test-token-0123456789";

pub(crate) fn config() -> Config {
    Config {
        key: KEY.into(),
        token: TOKEN.into(),
        model: super::MODEL.into(),
        fast_model: super::FAST_MODEL.into(),
        models: vec![super::MODEL.into(), super::FAST_MODEL.into(), "gemini-3.6-flash".into()],
        saved_at: 1,
    }
}

// A fresh empty folder of its own for each test.
pub(crate) fn temp_dir(tag: &str) -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let d = std::env::temp_dir().join(format!("cud-gemini-{tag}-{}-{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

pub(crate) fn app(dir: &Path, port: u16, cfg: Option<Config>) -> Arc<App> {
    let app = Arc::new(App {
        st: Mutex::new(State::default()),
        demo: false,
        data_file: dir.join("state.json"),
        creds_file: dir.join(".credentials.json"),
        claude_json: dir.join(".claude.json"),
        keychain: false,
        http: ureq::AgentBuilder::new().timeout(Duration::from_secs(5)).build(),
        polling: AtomicBool::new(false),
        last_poll: AtomicI64::new(0),
        seq: AtomicU64::new(0),
        port,
        desktop_dir: None,
        desktop_store: desktop::Store { root: dir.join("desktop") },
        switching: AtomicBool::new(false),
        login: Mutex::new(None),
        desktop_expect: Mutex::new(None),
        update: Mutex::new(update::Status::default()),
        gemini_sigs: Default::default(),
    });
    app.st.lock().unwrap().gemini = cfg;
    app
}

// The dashboard's own request handler (server.rs) on 127.0.0.1:<free port>.
pub(crate) fn server(cfg: Option<Config>) -> (Arc<App>, u16) {
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let port = server.server_addr().to_ip().unwrap().port();
    let app = app(&temp_dir("srv"), port, cfg);
    let a = app.clone();
    thread::spawn(move || {
        for rq in server.incoming_requests() {
            let a = a.clone();
            thread::spawn(move || handle(&a, rq));
        }
    });
    (app, port)
}

// One request; any status comes back as (status, body), not as an error.
pub(crate) fn call(port: u16, method: &str, path: &str, headers: &[(&str, &str)], body: Option<&str>) -> (u16, String) {
    let mut rq = ureq::request(method, &format!("http://127.0.0.1:{port}{path}"));
    for (k, v) in headers {
        rq = rq.set(k, v);
    }
    let r = match body {
        Some(b) => rq.send_string(b),
        None => rq.call(),
    };
    match r {
        Ok(r) | Err(ureq::Error::Status(_, r)) => (r.status(), r.into_string().unwrap_or_default()),
        Err(e) => panic!("{e}"),
    }
}
