// A saved sign-in whose claude.ai session has ended (docs/specs/desktop-signed-out.md):
// the store reports it, and leaving a signed-out app never overwrites a live one.

use super::cookie_session::chromium_time;
use super::*;
use serde_json::json;
use std::path::PathBuf;

const A: &str = "aaaaaaaa-0000-0000-0000-00000000000a";
const B: &str = "bbbbbbbb-0000-0000-0000-00000000000b";
const NOW: i64 = 1_762_920_000_000;

struct Quiet;
impl App for Quiet {
    fn stop(&self) -> Result<(), String> {
        Ok(())
    }
    fn start(&self) -> Result<(), String> {
        Ok(())
    }
}

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("cud-signed-out-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(d.join("app").join("Network")).unwrap();
    d
}

// A cookie store in `dir` with a claude.ai sessionKey worth `value`, or none.
fn cookies(dir: &Path, session: Option<&str>) {
    fs::create_dir_all(dir).unwrap();
    let _ = fs::remove_file(dir.join("Cookies"));
    let db = rusqlite::Connection::open(dir.join("Cookies")).unwrap();
    db.execute_batch("CREATE TABLE cookies (host_key TEXT, name TEXT, value TEXT, expires_utc INTEGER)").unwrap();
    db.execute("INSERT INTO cookies VALUES ('.claude.ai', 'anthropic-device-id', 'd', 0)", []).unwrap();
    if let Some(v) = session {
        db.execute("INSERT INTO cookies VALUES ('.claude.ai', 'sessionKey', ?1, ?2)", rusqlite::params![v, chromium_time(NOW + 86_400_000)]).unwrap();
    }
}

fn auth(dir: &Path, account: &str, token: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(dir.join("auth.json"), json!({ "oauth:tokenCache": token, "lastKnownAccountUuid": account }).to_string()).unwrap();
}

fn session_value(dir: &Path) -> Option<String> {
    let db = rusqlite::Connection::open(dir.join("Cookies")).ok()?;
    db.query_row("SELECT value FROM cookies WHERE name = 'sessionKey'", [], |r| r.get(0)).ok()
}

#[test]
fn the_store_tells_an_expired_sign_in_from_a_live_or_unknown_one() {
    let d = tmp("store");
    let store = Store { root: d.join("store") };
    auth(&store.profile(A), A, "tA");
    cookies(&store.profile(A), Some("live"));
    auth(&store.profile(B), B, "tB");
    cookies(&store.profile(B), None);
    assert!(!store.sign_in_expired(A, NOW) && store.usable(A, NOW));
    assert!(store.sign_in_expired(B, NOW) && !store.usable(B, NOW));
    // No cookie store saved, or none of Chromium's: unknown, so still usable.
    let c = "cccccccc-0000-0000-0000-00000000000c";
    auth(&store.profile(c), c, "tC");
    assert!(!store.sign_in_expired(c, NOW) && store.usable(c, NOW));
    // No saved sign-in at all: not usable, and not "expired" either.
    let none = "dddddddd-0000-0000-0000-00000000000d";
    assert!(!store.sign_in_expired(none, NOW) && !store.usable(none, NOW));
}

#[test]
fn the_store_tells_until_when_a_saved_sign_in_lasts() {
    let d = tmp("until");
    let store = Store { root: d.join("store") };
    auth(&store.profile(A), A, "tA");
    cookies(&store.profile(A), Some("live"));
    assert_eq!(store.saved_session(A, NOW), Some(Session::Live(Some(NOW + 86_400_000))));
    auth(&store.profile(B), B, "tB");
    cookies(&store.profile(B), None);
    assert_eq!(store.saved_session(B, NOW), Some(Session::Ended));
    assert_eq!(store.saved_session("cccccccc-0000-0000-0000-00000000000c", NOW), None, "none saved");
}

#[test]
fn leaving_a_signed_out_app_keeps_the_live_saved_sign_in() {
    let d = tmp("guard");
    let (app, store) = (d.join("app"), Store { root: d.join("store") });
    // A's saved sign-in is live; the app holds A's sign-in but its session has ended.
    auth(&store.profile(A), A, "tA-good");
    cookies(&store.profile(A), Some("A-live"));
    auth(&store.profile(B), B, "tB");
    cookies(&store.profile(B), Some("B-live"));
    fs::write(app.join("config.json"), json!({ "oauth:tokenCache": "tA-dead", "lastKnownAccountUuid": A }).to_string()).unwrap();
    cookies(&app.join("Network"), None);
    switch(&app, &store, Some(B), &Quiet, NOW, None).unwrap();
    assert_eq!(session_value(&store.profile(A)).as_deref(), Some("A-live"));
    assert!(fs::read_to_string(store.profile(A).join("auth.json")).unwrap().contains("tA-good"));
    assert_eq!(session_value(&app.join("Network")).as_deref(), Some("B-live"));
}

#[test]
fn leaving_a_live_app_saves_it_as_before() {
    let d = tmp("live");
    let (app, store) = (d.join("app"), Store { root: d.join("store") });
    auth(&store.profile(A), A, "tA-old");
    cookies(&store.profile(A), None);
    auth(&store.profile(B), B, "tB");
    cookies(&store.profile(B), Some("B-live"));
    fs::write(app.join("config.json"), json!({ "oauth:tokenCache": "tA-new", "lastKnownAccountUuid": A }).to_string()).unwrap();
    cookies(&app.join("Network"), Some("A-new"));
    switch(&app, &store, Some(B), &Quiet, NOW, None).unwrap();
    assert_eq!(session_value(&store.profile(A)).as_deref(), Some("A-new"));
    assert!(fs::read_to_string(store.profile(A).join("auth.json")).unwrap().contains("tA-new"));
}
