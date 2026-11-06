// What GitHub's answer starts: the 3 days, or a retry at the next hourly look
// (docs/specs/update.md, "When").

use super::*;

fn app(auto_update: bool) -> App {
    static N: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!("cud-upd-{}-{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
    let _ = fs::remove_dir_all(&dir);
    let app = App {
        st: Mutex::new(State::default()),
        demo: false,
        data_file: dir.join("state.json"),
        creds_file: dir.join(".credentials.json"),
        claude_json: dir.join(".claude.json"),
        keychain: false,
        http: ureq::AgentBuilder::new().build(),
        polling: AtomicBool::new(false),
        last_poll: AtomicI64::new(0),
        seq: AtomicU64::new(0),
        port: 0,
        desktop_dir: None,
        desktop_store: desktop::Store { root: dir.join("desktop") },
        switching: AtomicBool::new(false), desktop_auto_at: AtomicI64::new(0),
        login: Mutex::new(None),
        desktop_expect: Mutex::new(None),
        update: Mutex::new(Status::default()),
        gemini_sigs: Default::default(),
        gemini_watch: Default::default(),
        usage_log: Default::default(),
    };
    app.st.lock().unwrap().settings.auto_update = auto_update;
    app
}

fn release(tag: &str) -> Result<Value, String> {
    Ok(json!({"tag_name": tag, "html_url": format!("https://github.com/o/r/releases/tag/{tag}")}))
}

const DAY: i64 = 24 * 3_600_000;

#[test]
fn a_newer_version_to_install_keeps_the_next_look_due() {
    let a = app(true);
    a.st.lock().unwrap().update_checked_at = Some(DAY);
    a.take_answer(release("v999.0.0"), 10 * DAY).unwrap();
    assert_eq!(a.update.lock().unwrap().phase, Phase::Available);
    let last = a.st.lock().unwrap().update_checked_at;
    assert_eq!(last, Some(DAY), "not saved: the install is tried again");
    assert!(check_due(last, 10 * DAY + 3_600_000), "the next hourly look checks and installs");
}

#[test]
fn up_to_date_or_auto_update_off_starts_the_3_days() {
    let a = app(true);
    a.take_answer(release(&format!("v{VERSION}")), 10 * DAY).unwrap();
    assert_eq!(a.st.lock().unwrap().update_checked_at, Some(10 * DAY));
    let a = app(false);
    a.take_answer(release("v999.0.0"), 10 * DAY).unwrap();
    assert_eq!(a.update.lock().unwrap().phase, Phase::Available);
    assert_eq!(a.st.lock().unwrap().update_checked_at, Some(10 * DAY), "installed by hand");
}

#[test]
fn a_failed_check_saves_nothing() {
    let a = app(true);
    assert!(a.take_answer(Err("Could not reach GitHub".into()), 10 * DAY).is_err());
    assert_eq!(a.st.lock().unwrap().update_checked_at, None);
    assert_eq!(a.update.lock().unwrap().phase, Phase::Error);
}
