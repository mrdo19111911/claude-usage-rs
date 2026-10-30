// The usage log (docs/specs/usage-log.md, "Test first"): when a line is written, the
// seed from state.json's history, the 60 days.

use super::*;

const MIN: i64 = 60_000;
const DAY: i64 = 24 * HOUR;
const T: i64 = 1_793_000_000_000;

pub(crate) fn temp_dir(tag: &str) -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let d = std::env::temp_dir().join(format!("cud-ulog-{tag}-{}-{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

pub(crate) fn account(id: &str, five: i64, week: i64, history: Vec<(i64, i64, i64)>) -> Account {
    Account {
        id: id.into(),
        account_uuid: None,
        email: format!("{id}@example.com"),
        plan: "Max 5x".into(),
        enabled: true,
        creds: json!({"accessToken": "tok-never-logged"}),
        oauth_account: Value::Null,
        usage: Some(Usage { five: Win { u: five, resets_at: Some(T + HOUR) }, week: Win { u: week, resets_at: Some(T + 3 * DAY) }, fetched_at: T, scoped: vec![Scoped { name: "Fable".into(), u: 40, resets_at: Some(T + 3 * DAY) }] }),
        history,
        error: None,
        raw: json!({"secret": "raw-never-logged"}),
        slow_until: 0,
        slow_strikes: 0,
        next_check: 0,
    }
}

fn lines(dir: &Path) -> Vec<Value> {
    let text = fs::read_to_string(dir.join(FILE)).unwrap_or_default();
    text.lines().map(|l| serde_json::from_str(l).unwrap()).collect()
}

fn started(dir: &Path, st: &State, t: i64) -> Log {
    let log = Log::default();
    log.start(dir, st, t).unwrap();
    log
}

#[test]
fn a_line_per_new_answer_not_the_same_one_again_within_10_minutes() {
    let dir = temp_dir("write");
    let log = started(&dir, &State::default(), T);
    let a = account("a1", 13, 54, vec![]);
    assert!(log.append(figures(&a, true).unwrap(), T).unwrap(), "new answer: written");
    assert!(!log.append(figures(&a, true).unwrap(), T + 15_000).unwrap(), "same figures 15 s later: nothing");
    assert!(!log.append(figures(&a, true).unwrap(), T + 9 * MIN).unwrap());
    assert!(log.append(figures(&a, true).unwrap(), T + 10 * MIN).unwrap(), "10 minutes since: written");
    let b = account("a1", 14, 54, vec![]);
    assert!(log.append(figures(&b, true).unwrap(), T + 11 * MIN).unwrap(), "a changed %: written");
    let mut c = b.clone();
    c.usage.as_mut().unwrap().five.resets_at = Some(T + 2 * HOUR);
    assert!(log.append(figures(&c, true).unwrap(), T + 12 * MIN).unwrap(), "a changed reset: written");
    assert!(log.append(figures(&c, false).unwrap(), T + 13 * MIN).unwrap(), "active changed: written");
    let other = account("a2", 14, 54, vec![]);
    assert!(log.append(figures(&other, false).unwrap(), T + 13 * MIN).unwrap(), "another account: its own last line");

    let got = lines(&dir);
    assert_eq!(got.len(), 6);
    assert_eq!(got[0], json!({"t": T, "id": "a1", "plan": "Max 5x", "active": true,
        "five": {"u": 13, "resetsAt": T + HOUR}, "week": {"u": 54, "resetsAt": T + 3 * DAY},
        "scoped": [{"name": "Fable", "u": 40, "resetsAt": T + 3 * DAY}]}));
    let text = fs::read_to_string(dir.join(FILE)).unwrap();
    for bad in ["example.com", "tok-never-logged", "raw-never-logged"] {
        assert!(!text.contains(bad), "{bad} in the log");
    }
    let s = log.stats();
    assert_eq!((s["lines"].as_i64(), s["since"].as_i64(), s["bytes"].as_u64()), (Some(6), Some(T), Some(text.len() as u64)));
}

#[test]
fn a_restart_remembers_each_accounts_last_line() {
    let dir = temp_dir("restart");
    let a = account("a1", 13, 54, vec![]);
    started(&dir, &State::default(), T).append(figures(&a, true).unwrap(), T).unwrap();
    let log = started(&dir, &State::default(), T + MIN);
    assert!(!log.append(figures(&a, true).unwrap(), T + 2 * MIN).unwrap(), "same figures after a restart: nothing");
}

#[test]
fn state_history_seeds_the_file_once() {
    let dir = temp_dir("seed");
    let st = State { accounts: vec![account("a1", 0, 0, vec![(T - 2 * HOUR, 5, 20), (T - HOUR, 7, 21)]), account("a2", 0, 0, vec![(T - 90 * MIN, 1, 2)])], ..Default::default() };
    let log = started(&dir, &st, T);
    let got = lines(&dir);
    let ts: Vec<i64> = got.iter().map(|l| l["t"].as_i64().unwrap()).collect();
    assert_eq!(ts, vec![T - 2 * HOUR, T - 90 * MIN, T - HOUR], "in time order");
    assert_eq!(got[1], json!({"t": T - 90 * MIN, "id": "a2", "plan": null, "active": false,
        "five": {"u": 1, "resetsAt": null}, "week": {"u": 2, "resetsAt": null}, "scoped": [], "seed": true}));
    assert_eq!(log.stats()["lines"], 3);
    drop(log);
    started(&dir, &st, T + MIN);
    assert_eq!(lines(&dir).len(), 3, "a second start does not seed again");
}

#[test]
fn lines_older_than_60_days_are_dropped_at_start() {
    let dir = temp_dir("trim");
    let mk = |t: i64, u: i64| json!({"t": t, "id": "a1", "plan": "Pro", "active": false, "five": {"u": u, "resetsAt": null}, "week": {"u": u, "resetsAt": null}, "scoped": []}).to_string();
    let old = [mk(T - 70 * DAY, 1), mk(T - 61 * DAY, 2)];
    let new = [mk(T - 59 * DAY, 3), mk(T - DAY, 4), mk(T - MIN, 5)];
    fs::write(dir.join(FILE), format!("{}\n{}\n{}\n{}\n{}\n", old[0], old[1], new[0], new[1], new[2])).unwrap();
    let log = started(&dir, &State::default(), T);
    let us: Vec<i64> = lines(&dir).iter().map(|l| l["five"]["u"].as_i64().unwrap()).collect();
    assert_eq!(us, vec![3, 4, 5], "newer ones stay, in order");
    assert_eq!(log.stats()["since"], T - 59 * DAY);
    assert!(!dir.join("usage-log.cud.tmp").exists(), "rewritten through a temp file, then renamed");
}

#[test]
fn a_failed_write_is_logged_once_and_polling_goes_on() {
    let dir = temp_dir("fail");
    let app = test_app(&dir);
    app.start_usage_log(&mut app.st.lock().unwrap());
    fs::create_dir_all(dir.join(FILE)).unwrap(); // the file cannot be opened for writing
    for u in 1..=3 {
        app.log_usage(figures(&account("a1", u, 1, vec![]), false).unwrap());
    }
    let st = app.st.lock().unwrap();
    let said: Vec<&Event> = st.events.iter().filter(|e| e.msg.starts_with("Usage log: could not write")).collect();
    assert_eq!(said.len(), 1, "logged once");
}

#[test]
fn the_demo_writes_no_log() {
    let dir = temp_dir("demo");
    let mut app = test_app(&dir);
    app.demo = true;
    let mut st = State { accounts: vec![account("a1", 1, 1, vec![(T, 1, 1)])], ..Default::default() };
    app.start_usage_log(&mut st);
    app.log_usage(figures(&st.accounts[0], true).unwrap());
    assert!(!dir.join(FILE).exists());
    assert_eq!(app.usage_log.stats(), json!({"lines": 0, "since": null, "bytes": 0}));
}

// An App in `dir`, as main builds it (no network is used by these tests).
pub(crate) fn test_app(dir: &Path) -> App {
    App {
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
        port: 0,
        desktop_dir: None,
        desktop_store: desktop::Store { root: dir.join("desktop") },
        switching: AtomicBool::new(false),
        login: Mutex::new(None),
        desktop_expect: Mutex::new(None),
        update: Mutex::new(update::Status::default()),
        gemini_sigs: Default::default(),
        gemini_watch: Default::default(),
        usage_log: Default::default(),
    }
}
