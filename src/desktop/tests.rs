use super::*;
use serde_json::json;
use std::cell::RefCell;

const A: &str = "aaaaaaaa-0000-0000-0000-000000000001";
const B: &str = "bbbbbbbb-0000-0000-0000-000000000002";

struct Fake {
    calls: RefCell<Vec<&'static str>>,
    fail_stop: bool,
}
impl App for Fake {
    fn stop(&self) -> Result<(), String> {
        self.calls.borrow_mut().push("stop");
        if self.fail_stop { Err("stuck".into()) } else { Ok(()) }
    }
    fn start(&self) -> Result<(), String> {
        self.calls.borrow_mut().push("start");
        Ok(())
    }
}
fn fake() -> Fake {
    Fake { calls: RefCell::new(vec![]), fail_stop: false }
}

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("cud-test-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(d.join("app").join("Network")).unwrap();
    d
}

fn sign_in(app: &Path, account: &str, token: &str, cookie: &str) {
    let cfg = json!({ "zFirst": 1, "oauth:tokenCache": format!("v10{token}"), "lastKnownAccountUuid": account, "dxt:allowlist": true, "oauth:tokenCacheV2": "v10x" });
    fs::write(app.join("config.json"), cfg.to_string()).unwrap();
    fs::write(app.join("Network").join("Cookies"), cookie).unwrap();
}

fn config(app: &Path) -> Map<String, Value> {
    read_config(app).unwrap()
}

#[test]
fn switches_back_and_forth_keeping_other_settings_and_key_order() {
    let d = tmp("swap");
    let (app, store) = (d.join("app"), Store { root: d.join("store") });
    sign_in(&app, A, "tokA", "cookieA");
    // First switch to a new account: A is saved, the app is left signed out.
    assert_eq!(switch(&app, &store, None, &fake(), 1, None).unwrap().saved.as_deref(), Some(A));
    assert!(store.has_sign_in(A));
    assert_eq!(signed_in(&app), None);
    assert!(!app.join("Network").join("Cookies").exists());
    assert_eq!(config(&app).get("dxt:allowlist"), Some(&json!(true)));
    // The person signs in to B in the app; switching to A saves B and restores A.
    sign_in(&app, B, "tokB", "cookieB");
    let f = fake();
    assert_eq!(switch(&app, &store, Some(A), &f, 2, None).unwrap().saved.as_deref(), Some(B));
    assert_eq!(*f.calls.borrow(), vec!["stop", "start"]);
    assert_eq!(signed_in(&app).as_deref(), Some(A));
    assert_eq!(config(&app)["oauth:tokenCache"], json!("v10tokA"));
    assert_eq!(fs::read_to_string(app.join("Network").join("Cookies")).unwrap(), "cookieA");
    let keys: Vec<String> = config(&app).keys().cloned().collect();
    assert_eq!(keys.first().map(String::as_str), Some("zFirst"), "untouched keys keep their order");
    // And back to B.
    switch(&app, &store, Some(B), &fake(), 3, None).unwrap();
    assert_eq!(signed_in(&app).as_deref(), Some(B));
    assert_eq!(fs::read_to_string(app.join("Network").join("Cookies")).unwrap(), "cookieB");
}

#[test]
fn saves_the_sign_in_as_it_is_after_the_app_closed() {
    // The app refreshes its token while quitting: the saved copy must be that one.
    struct Rotating(PathBuf);
    impl App for Rotating {
        fn stop(&self) -> Result<(), String> {
            sign_in(&self.0, A, "tokA-new", "cookieA");
            Ok(())
        }
        fn start(&self) -> Result<(), String> {
            Ok(())
        }
    }
    let d = tmp("rotate");
    let (app, store) = (d.join("app"), Store { root: d.join("store") });
    sign_in(&app, A, "tokA-old", "cookieA");
    switch(&app, &store, None, &Rotating(app.clone()), 1, None).unwrap();
    let saved = fs::read_to_string(store.profile(A).join("auth.json")).unwrap();
    assert!(saved.contains("tokA-new") && !saved.contains("tokA-old"));
}

#[test]
fn refuses_unknown_or_bad_targets_without_closing_the_app() {
    let d = tmp("refuse");
    let (app, store) = (d.join("app"), Store { root: d.join("store") });
    sign_in(&app, A, "tokA", "cookieA");
    let f = fake();
    assert!(switch(&app, &store, Some(B), &f, 1, None).is_err());
    assert!(switch(&app, &store, Some("..\\..\\x"), &f, 1, None).is_err());
    assert!(f.calls.borrow().is_empty());
}

#[test]
fn a_failed_close_leaves_the_sign_in_alone_and_restarts() {
    let d = tmp("stuck");
    let (app, store) = (d.join("app"), Store { root: d.join("store") });
    sign_in(&app, A, "tokA", "cookieA");
    switch(&app, &store, None, &fake(), 1, None).unwrap();
    sign_in(&app, B, "tokB", "cookieB");
    let f = Fake { calls: RefCell::new(vec![]), fail_stop: true };
    assert!(switch(&app, &store, Some(A), &f, 2, None).is_err());
    assert_eq!(*f.calls.borrow(), vec!["stop", "start"]);
    assert_eq!(signed_in(&app).as_deref(), Some(B));
}

#[test]
fn keeps_the_last_backups_only() {
    let d = tmp("backups");
    let (app, store) = (d.join("app"), Store { root: d.join("store") });
    sign_in(&app, A, "tokA", "cookieA");
    for i in 0..8 {
        store.backup(&app, i).unwrap();
    }
    let left: Vec<String> = subdirs(&store.root.join("backups"), "").iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect();
    assert_eq!(left, vec!["3", "4", "5", "6", "7"]);
}

#[test]
fn finds_the_relocated_data_folder() {
    let d = tmp("find");
    let local = d.join("local");
    let moved = local.join("Claude-Data").join("claude_pzs8sxrjxfjjc");
    fs::create_dir_all(&moved).unwrap();
    fs::write(moved.join("config.json"), "{}").unwrap();
    fs::create_dir_all(local.join("Packages").join("Claude_pzs8sxrjxfjjc").join("LocalCache").join("Roaming").join("Claude")).unwrap();
    assert_eq!(subdirs(&local.join("Claude-Data"), "claude_"), vec![moved]);
}

// docs/specs/desktop-sessions.md: the sync sits between closing and starting the app.
fn with_a_card_in_a(app: &Path) -> PathBuf {
    let org = "0000000a-1111-4000-8000-00000000000f";
    let root = app.join("claude-code-sessions");
    fs::create_dir_all(root.join(A).join(org)).unwrap();
    fs::create_dir_all(root.join(B).join(org)).unwrap();
    fs::write(root.join(A).join(org).join("local_x.json"), r#"{"sessionId":"s-x"}"#).unwrap();
    root.join(B).join(org).join("local_x.json")
}

#[test]
fn the_setting_off_shares_nothing_and_on_shares_before_the_start() {
    let d = tmp("share");
    let (app, store) = (d.join("app"), Store { root: d.join("store") });
    sign_in(&app, A, "tokA", "cookieA");
    let in_b = with_a_card_in_a(&app);
    let done = switch(&app, &store, None, &fake(), 1, None).unwrap();
    assert!(done.shared.is_none() && !in_b.exists());
    struct Check(PathBuf);
    impl App for Check {
        fn stop(&self) -> Result<(), String> {
            Ok(())
        }
        fn start(&self) -> Result<(), String> {
            assert!(self.0.is_file(), "shared before the app starts");
            Ok(())
        }
    }
    sign_in(&app, B, "tokB", "cookieB");
    let done = switch(&app, &store, Some(A), &Check(in_b.clone()), 2, Some(&d.join("state"))).unwrap();
    assert_eq!(done.shared.unwrap().unwrap(), sessions::Counts { added: 1, updated: 0, removed: 0 });
}

#[test]
fn a_sync_error_still_switches_and_starts_the_app() {
    let d = tmp("share-fail");
    let (app, store) = (d.join("app"), Store { root: d.join("store") });
    sign_in(&app, A, "tokA", "cookieA");
    with_a_card_in_a(&app);
    // The state folder's place is taken by a file: the backup cannot be made.
    fs::write(d.join("state"), "x").unwrap();
    let f = fake();
    let done = switch(&app, &store, None, &f, 1, Some(&d.join("state"))).unwrap();
    assert!(done.shared.unwrap().is_err());
    assert_eq!(*f.calls.borrow(), vec!["stop", "start"]);
    assert_eq!(signed_in(&app), None);
}

// docs/REVIEW_DE_HIEU.md section 5: shown after "Claude Desktop switch failed:".
#[cfg(windows)]
#[test]
fn the_real_app_errors_start_with_a_capital() {
    assert_eq!(super::win::NOT_CLOSED, "Claude Desktop did not close");
    assert_eq!(super::win::NOT_STARTED, "Could not start Claude Desktop; start it from the Start menu");
}
