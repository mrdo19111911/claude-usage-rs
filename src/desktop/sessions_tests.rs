// docs/specs/desktop-sessions.md, "Test first": temp folders only, never the real Desktop.

use super::*;
use serde_json::json;
use std::fs;

// Changes since the last sync (docs/specs/desktop-sessions.md, "Update").
#[path = "sessions_3way_tests.rs"]
mod three_way;

const A: &str = "aaaaaaaa-0000-4000-8000-000000000001";
const B: &str = "bbbbbbbb-0000-4000-8000-000000000002";
const C: &str = "cccccccc-0000-4000-8000-000000000003";
const ORG: &str = "0000000a-1111-4000-8000-00000000000f";

struct T {
    app: PathBuf,
    state: PathBuf,
}

fn t(name: &str) -> T {
    let d = std::env::temp_dir().join(format!("cud-sessions-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    let t = T { app: d.join("app"), state: d.join("data").join("desktop-sessions") };
    for acc in [A, B] {
        fs::create_dir_all(t.folder(acc)).unwrap();
    }
    t
}

impl T {
    fn folder(&self, acc: &str) -> PathBuf {
        self.app.join("claude-code-sessions").join(acc).join(ORG)
    }
    fn put(&self, acc: &str, name: &str, card: Value) {
        fs::create_dir_all(self.folder(acc)).unwrap();
        fs::write(self.folder(acc).join(name), card.to_string()).unwrap();
    }
    fn raw(&self, acc: &str, name: &str) -> Option<String> {
        fs::read_to_string(self.folder(acc).join(name)).ok()
    }
    fn card(&self, acc: &str, name: &str) -> Option<Value> {
        self.raw(acc, name).map(|s| serde_json::from_str(&s).unwrap())
    }
    fn sync(&self, stamp: i64) -> Counts {
        sync(&self.app, &self.state, stamp).unwrap()
    }
}

fn counts(added: usize, updated: usize, removed: usize) -> Counts {
    Counts { added, updated, removed }
}

#[test]
fn a_card_only_in_a_reaches_b_without_account_keys_and_with_bs_connectors() {
    let t = t("add");
    t.put(A, "local_x.json", json!({"sessionId": "s-x", "title": "X", "lastActivityAt": 10, "remoteMcpServersConfig": "mcp-A",
        "enabledMcpTools": "tools-A", "toolSurfaceSnapshot": "snap-A", "bridgeSessionIds": ["b"], "error": "e", "errorAt": 1}));
    t.put(B, "local_n.json", json!({"sessionId": "s-n", "lastActivityAt": 5, "remoteMcpServersConfig": "mcp-B",
        "enabledMcpTools": "tools-B", "toolSurfaceSnapshot": "snap-B"}));
    assert_eq!(t.sync(1), counts(2, 0, 0));
    let x = t.card(B, "local_x.json").unwrap();
    assert_eq!(x["title"], "X");
    assert_eq!((&x["remoteMcpServersConfig"], &x["enabledMcpTools"], &x["toolSurfaceSnapshot"]), (&json!("mcp-B"), &json!("tools-B"), &json!("snap-B")));
    for k in ["bridgeSessionIds", "error", "errorAt"] {
        assert!(x.get(k).is_none(), "{k} stays with A");
    }
    assert_eq!(t.card(A, "local_n.json").unwrap()["remoteMcpServersConfig"], "mcp-A");
    // Nothing left to do the second time.
    assert_eq!(t.sync(2), counts(0, 0, 0));
}

#[test]
fn a_fresher_card_updates_the_others_keeping_their_account_keys() {
    let t = t("update");
    t.put(A, "local_x.json", json!({"sessionId": "s-x", "title": "old", "lastActivityAt": 10, "remoteMcpServersConfig": "mcp-A", "error": "eA"}));
    t.put(B, "local_x.json", json!({"sessionId": "s-x", "title": "new", "lastFocusedAt": 20, "remoteMcpServersConfig": "mcp-B"}));
    assert_eq!(t.sync(1), counts(0, 1, 0));
    let x = t.card(A, "local_x.json").unwrap();
    assert_eq!((&x["title"], &x["lastFocusedAt"]), (&json!("new"), &json!(20)));
    assert_eq!((&x["remoteMcpServersConfig"], &x["error"]), (&json!("mcp-A"), &json!("eA")));
    assert_eq!(t.card(B, "local_x.json").unwrap()["remoteMcpServersConfig"], "mcp-B");
}

#[test]
fn a_card_deleted_in_a_since_the_last_sync_goes_to_trash_in_b() {
    let t = t("delete");
    t.put(A, "local_x.json", json!({"sessionId": "s-x", "lastActivityAt": 10}));
    t.put(A, "local_k.json", json!({"sessionId": "s-k", "lastActivityAt": 3}));
    t.sync(1);
    assert!(t.card(B, "local_x.json").is_some());
    fs::remove_file(t.folder(A).join("local_x.json")).unwrap();
    assert_eq!(t.sync(1_700_000_000_000), counts(0, 0, 1));
    assert!(t.card(B, "local_x.json").is_none());
    assert!(t.card(B, "local_k.json").is_some());
    let trash = t.state.join("trash").join(format!("{}-{}", &B[..8], &ORG[..8])).join("local_x.json.1700000000000");
    assert!(trash.is_file(), "moved, not deleted");
}

#[test]
fn a_folder_that_cannot_be_read_deletes_nothing() {
    let t = t("unreadable");
    t.put(A, "local_x.json", json!({"sessionId": "s-x", "lastActivityAt": 10}));
    t.put(C, "local_c.json", json!({"sessionId": "s-c", "lastActivityAt": 4}));
    t.sync(1);
    let root = t.app.join("claude-code-sessions");
    let mut folders = load(&root);
    let a = folders.iter_mut().find(|f| f.account == A).unwrap();
    (a.readable, a.cards, a.present) = (false, Default::default(), Default::default());
    let mut st = SyncState::load(&t.state);
    assert_eq!(run(&mut folders, &mut st, &t.state, 2).unwrap(), counts(0, 0, 0));
    assert!(t.card(B, "local_x.json").is_some() && t.card(C, "local_x.json").is_some());
    // Its last list is kept for when it can be read again.
    assert_eq!(st.seen[&format!("{A}/{ORG}")], vec!["local_c.json", "local_x.json"]);
}

#[test]
fn an_unreadable_card_is_untouched_everywhere_and_never_copied() {
    let t = t("badcard");
    fs::write(t.folder(A).join("local_bad.json"), "{ not json").unwrap();
    t.put(A, "local_noid.json", json!({"title": "no sessionId"}));
    t.put(A, "local_x.json", json!({"sessionId": "s-x", "lastActivityAt": 10}));
    fs::write(t.folder(B).join("local_x.json"), "half-writ").unwrap();
    t.sync(1);
    assert!(t.raw(B, "local_bad.json").is_none() && t.raw(B, "local_noid.json").is_none());
    assert_eq!(t.raw(B, "local_x.json").as_deref(), Some("half-writ"));
    assert_eq!(t.raw(A, "local_bad.json").as_deref(), Some("{ not json"));
    // Gone from A later: B's unreadable one is still not touched.
    fs::remove_file(t.folder(A).join("local_x.json")).unwrap();
    t.sync(2);
    assert_eq!(t.raw(B, "local_x.json").as_deref(), Some("half-writ"));
}

#[test]
fn the_archive_index_follows_is_archived_and_other_files_stay() {
    let t = t("index");
    t.put(A, "local_x.json", json!({"sessionId": "s-x", "isArchived": true, "lastActivityAt": 2}));
    t.put(A, "local_y.json", json!({"sessionId": "s-y", "isArchived": false, "lastActivityAt": 3}));
    fs::write(t.folder(B).join("archived-sessions.idx"), r#"{"v":1,"archived":["s-old"]}"#).unwrap();
    for (f, text) in [("backlog", "keep"), ("waiting-input", "w"), ("scheduled-tasks.json", "[1]")] {
        fs::write(t.folder(B).join(f), text).unwrap();
    }
    t.sync(1);
    for acc in [A, B] {
        assert_eq!(t.raw(acc, "archived-sessions.idx").as_deref(), Some(r#"{"v":1,"archived":["s-x"]}"#));
    }
    for (f, text) in [("backlog", "keep"), ("waiting-input", "w"), ("scheduled-tasks.json", "[1]")] {
        assert_eq!(t.raw(B, f).as_deref(), Some(text));
    }
    let names: Vec<String> = fs::read_dir(t.folder(B)).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    assert!(names.iter().all(|n| !n.contains(".tmp")), "no temp file left: {names:?}");
}

#[test]
fn the_first_sync_makes_the_backup_and_the_second_does_not() {
    let t = t("backup");
    t.put(A, "local_x.json", json!({"sessionId": "s-x"}));
    fs::remove_dir_all(t.folder(B)).unwrap();
    // One folder only: nothing to do, no backup.
    assert_eq!(t.sync(1), counts(0, 0, 0));
    assert!(!t.state.exists());
    fs::create_dir_all(t.folder(B)).unwrap();
    t.sync(1_700_000_000_000);
    t.sync(1_700_000_100_000);
    let backups: Vec<String> = fs::read_dir(&t.state).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).filter(|n| n.starts_with("backup-")).collect();
    assert_eq!(backups, vec!["backup-20231114-221320"]);
    assert!(t.state.join(&backups[0]).join(A).join(ORG).join("local_x.json").is_file());
    assert!(!t.state.join(&backups[0]).join(B).join(ORG).join("local_x.json").exists(), "taken before the sync");
    let st: Value = serde_json::from_str(&fs::read_to_string(t.state.join("state.json")).unwrap()).unwrap();
    assert_eq!(st["backedUp"], true);
    assert_eq!(st["seen"][format!("{B}/{ORG}")], json!(["local_x.json"]));
    assert_eq!(st["copied"][format!("{B}/{ORG}")], json!(["local_x.json"]));
}

#[test]
fn only_uuid_folders_count() {
    let t = t("names");
    t.put(A, "local_x.json", json!({"sessionId": "s-x"}));
    fs::create_dir_all(t.app.join("claude-code-sessions").join("not-a-uuid").join(ORG)).unwrap();
    fs::create_dir_all(t.app.join("claude-code-sessions").join(B).join("other")).unwrap();
    t.sync(1);
    assert!(fs::read_dir(t.app.join("claude-code-sessions").join("not-a-uuid").join(ORG)).unwrap().next().is_none());
    assert!(fs::read_dir(t.app.join("claude-code-sessions").join(B).join("other")).unwrap().next().is_none());
}
