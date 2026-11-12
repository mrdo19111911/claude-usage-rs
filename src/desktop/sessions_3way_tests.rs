// docs/specs/desktop-sessions.md, "Update (changed since the last sync)": a change wins
// because it is new since the last sync, not because its times are later.

use super::*;
use crate::desktop::sessions_rules::mark;
use std::time::{Duration, SystemTime};

impl T {
    // The card file's modified time set `secs` before now.
    fn age(&self, acc: &str, name: &str, secs: u64) {
        let f = fs::File::options().write(true).open(self.folder(acc).join(name)).unwrap();
        f.set_modified(SystemTime::now() - Duration::from_secs(secs)).unwrap();
    }
    fn index(&self, acc: &str) -> String {
        fs::read_to_string(self.folder(acc).join("archived-sessions.idx")).unwrap_or_default()
    }
}

fn x(extra: Value) -> Value {
    let mut c = json!({"sessionId": "s-x", "title": "X", "lastActivityAt": 10, "lastFocusedAt": 12});
    c.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
    c
}

#[test]
fn an_archive_in_one_account_reaches_the_others_though_its_times_did_not_move() {
    let t = t("archive");
    t.put(A, "local_x.json", x(json!({})));
    t.put(B, "local_x.json", x(json!({})));
    assert_eq!(t.sync(1), counts(0, 0, 0));
    t.put(A, "local_x.json", x(json!({"isArchived": true})));
    assert_eq!(t.sync(2), counts(0, 1, 0));
    assert_eq!(t.card(B, "local_x.json").unwrap()["isArchived"], true);
    assert!(t.index(B).contains("s-x"), "B's archive index: {}", t.index(B));
    // Unarchived again in B: back to A.
    t.put(B, "local_x.json", x(json!({"isArchived": false})));
    assert_eq!(t.sync(3), counts(0, 1, 0));
    assert_eq!(t.card(A, "local_x.json").unwrap()["isArchived"], false);
}

#[test]
fn with_equal_times_and_no_earlier_sync_the_later_written_file_wins() {
    let t = t("tie");
    t.put(A, "local_x.json", x(json!({"title": "newer"})));
    t.put(B, "local_x.json", x(json!({"title": "older"})));
    t.age(A, "local_x.json", 60);
    t.age(B, "local_x.json", 3600);
    assert_eq!(t.sync(1), counts(0, 1, 0));
    assert_eq!(t.card(B, "local_x.json").unwrap()["title"], "newer");
}

#[test]
fn changed_in_both_the_later_activity_wins() {
    let t = t("both");
    t.put(A, "local_x.json", x(json!({})));
    t.put(B, "local_x.json", x(json!({})));
    t.sync(1);
    t.put(A, "local_x.json", x(json!({"title": "used in A", "lastActivityAt": 50})));
    t.put(B, "local_x.json", x(json!({"isArchived": true})));
    assert_eq!(t.sync(2), counts(0, 1, 0));
    let b = t.card(B, "local_x.json").unwrap();
    assert_eq!((&b["title"], &b["lastActivityAt"]), (&json!("used in A"), &json!(50)));
}

#[test]
fn a_write_that_failed_is_not_taken_for_a_change_next_time() {
    let t = t("failed");
    t.put(A, "local_x.json", x(json!({})));
    t.put(B, "local_x.json", x(json!({})));
    t.sync(1);
    t.put(A, "local_x.json", x(json!({"title": "new", "lastActivityAt": 50})));
    let bp = t.folder(B).join("local_x.json");
    let mut ro = fs::metadata(&bp).unwrap().permissions();
    ro.set_readonly(true);
    fs::set_permissions(&bp, ro.clone()).unwrap();
    assert!(sync(&t.app, &t.state, 2).is_err(), "B could not be written");
    #[allow(clippy::permissions_set_readonly_false)]
    ro.set_readonly(false);
    fs::set_permissions(&bp, ro).unwrap();
    assert_eq!(t.sync(3), counts(0, 1, 0));
    assert_eq!(t.card(A, "local_x.json").unwrap()["title"], "new", "B's old card did not win");
    assert_eq!(t.card(B, "local_x.json").unwrap()["title"], "new");
}

fn set_readonly(p: &Path, on: bool) {
    let mut perm = fs::metadata(p).unwrap().permissions();
    #[allow(clippy::permissions_set_readonly_false)]
    perm.set_readonly(on);
    fs::set_permissions(p, perm).unwrap();
}

#[test]
fn an_archive_not_copied_everywhere_still_wins_next_time_whatever_the_file_times() {
    let t = t("failed-archive");
    t.put(A, "local_x.json", x(json!({})));
    t.put(B, "local_x.json", x(json!({})));
    t.sync(1);
    t.put(A, "local_x.json", x(json!({"isArchived": true})));
    t.age(A, "local_x.json", 3600);
    let bp = t.folder(B).join("local_x.json");
    set_readonly(&bp, true);
    assert!(sync(&t.app, &t.state, 2).is_err());
    set_readonly(&bp, false);
    // B's untouched old card is now the later-written file: the archive must still win.
    assert_eq!(t.sync(3), counts(0, 1, 0));
    assert_eq!(t.card(B, "local_x.json").unwrap()["isArchived"], true);
    assert_eq!(t.card(A, "local_x.json").unwrap()["isArchived"], true);
}

#[test]
fn the_first_sync_by_marks_backs_up_again() {
    let t = t("marks-backup");
    t.put(A, "local_x.json", x(json!({})));
    fs::create_dir_all(&t.state).unwrap();
    fs::write(t.state.join("state.json"), r#"{"seen":{},"copied":{},"backedUp":true}"#).unwrap();
    t.sync(1_700_000_000_000);
    t.sync(1_700_000_100_000);
    let backups: Vec<String> = fs::read_dir(&t.state).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).filter(|n| n.starts_with("backup-")).collect();
    assert_eq!(backups, vec!["backup-20231114-221320"], "once, before the first sync that keeps marks");
}

#[test]
fn the_mark_ignores_key_order_and_the_account_keys() {
    let card = |v: Value| v.as_object().unwrap().clone();
    let a = card(json!({"sessionId": "s", "title": "T", "n": {"p": 1, "q": 2}, "remoteMcpServersConfig": "A"}));
    let b = card(json!({"n": {"q": 2, "p": 1}, "remoteMcpServersConfig": "B", "title": "T", "sessionId": "s", "error": "e"}));
    assert_eq!(mark(&a), mark(&b));
    assert_eq!(mark(&a).len(), 16);
    assert_ne!(mark(&a), mark(&card(json!({"sessionId": "s", "title": "U", "n": {"p": 1, "q": 2}}))));
}

#[test]
fn the_state_keeps_each_folders_marks() {
    let t = t("marks");
    t.put(A, "local_x.json", x(json!({})));
    t.sync(1);
    let st: Value = serde_json::from_str(&fs::read_to_string(t.state.join("state.json")).unwrap()).unwrap();
    for acc in [A, B] {
        let m = &st["marks"][format!("{acc}/{ORG}")]["local_x.json"];
        assert!(m.as_str().is_some_and(|s| s.len() == 16), "{acc}: {m}");
    }
}

// The rule on a copy of this computer's real folders and sync state (never the real ones):
// cargo test real_sessions_dry_run -- --ignored --nocapture
#[test]
#[ignore]
fn real_sessions_dry_run() {
    let home = PathBuf::from(std::env::var_os("USERPROFILE").unwrap());
    let real = crate::desktop::find_data_dir().unwrap();
    let t = t("real");
    let _ = fs::remove_dir_all(&t.app);
    crate::desktop::sessions_io::copy_tree(&real.join("claude-code-sessions"), &t.app.join("claude-code-sessions")).unwrap();
    fs::create_dir_all(&t.state).unwrap();
    fs::copy(home.join(".claude-usage-dashboard/desktop-sessions/state.json"), t.state.join("state.json")).unwrap();
    let shown = |t: &T| load(&t.app.join("claude-code-sessions")).iter().map(|f| (f.account[..8].to_string(), f.cards.values().filter(|c| c.get("isArchived") != Some(&json!(true))).count())).collect::<Vec<_>>();
    println!("before: {:?}", shown(&t));
    println!("first sync: {:?}", t.sync(1));
    println!("after: {:?}", shown(&t));
    println!("second sync: {:?}", t.sync(2));
}
