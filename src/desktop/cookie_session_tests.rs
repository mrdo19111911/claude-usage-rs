// Tests for cookie_session (spec: docs/specs/desktop-signed-out.md).

use super::cookie_session::{chromium_time, read_session, session_state, Session};
use std::fs;
use std::path::PathBuf;

const NOW: i64 = 1_762_920_000_000; // 2025-11-12, ms since 1970

fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("cud-cookie-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

// A cookie store with Chromium's columns that matter here, holding `rows`
// (host_key, name, expires_utc).
fn store(name: &str, rows: &[(&str, &str, i64)]) -> PathBuf {
    let p = dir(name).join("Cookies");
    let db = rusqlite::Connection::open(&p).unwrap();
    db.execute_batch("CREATE TABLE cookies (creation_utc INTEGER, host_key TEXT, name TEXT, value TEXT, encrypted_value BLOB, expires_utc INTEGER, has_expires INTEGER)").unwrap();
    for (h, n, e) in rows {
        db.execute("INSERT INTO cookies VALUES (0, ?1, ?2, '', x'00', ?3, ?4)", rusqlite::params![h, n, e, (*e != 0) as i64]).unwrap();
    }
    p
}

const DAY: i64 = 86_400_000;

#[test]
fn chromium_time_counts_microseconds_from_1601() {
    assert_eq!(chromium_time(0), 11_644_473_600_000_000);
    assert_eq!(chromium_time(1), 11_644_473_600_001_000);
}

#[test]
fn a_live_session_gives_its_latest_expiry() {
    let (soon, late) = (NOW + 3 * DAY, NOW + 27 * DAY);
    let p = store("until", &[(".claude.ai", "sessionKey", chromium_time(soon)), ("claude.ai", "sessionKey", chromium_time(late)), (".claude.ai", "sessionKey", chromium_time(NOW - DAY))]);
    assert_eq!(read_session(&p, NOW), Some(Session::Live(Some(late))));
    let s = store("until-none", &[(".claude.ai", "sessionKey", chromium_time(soon)), (".claude.ai", "sessionKey", 0)]);
    assert_eq!(read_session(&s, NOW), Some(Session::Live(None)), "a cookie without expiry lasts as long as it is kept");
    assert_eq!(read_session(&store("until-ended", &[]), NOW), Some(Session::Ended));
    assert_eq!(read_session(&s.with_file_name("missing"), NOW), None);
}

#[test]
fn a_live_session_key_is_signed_in() {
    let p = store("live", &[(".claude.ai", "sessionKey", chromium_time(NOW + 30 * DAY))]);
    assert_eq!(session_state(&p, NOW), Some(true));
}

#[test]
fn an_expired_session_key_is_signed_out() {
    let p = store("expired", &[(".claude.ai", "sessionKey", chromium_time(NOW - DAY))]);
    assert_eq!(session_state(&p, NOW), Some(false));
}

#[test]
fn a_session_key_without_expiry_is_signed_in() {
    let p = store("session", &[("claude.ai", "sessionKey", 0)]);
    assert_eq!(session_state(&p, NOW), Some(true));
}

#[test]
fn another_site_or_cookie_does_not_count() {
    let far = chromium_time(NOW + DAY);
    let p = store("other", &[(".notclaude.ai", "sessionKey", far), (".evil.com", "sessionKey", far), (".claude.ai", "anthropic-device-id", far)]);
    assert_eq!(session_state(&p, NOW), Some(false));
}

#[test]
fn an_empty_store_is_signed_out() {
    assert_eq!(session_state(&store("empty", &[]), NOW), Some(false));
}

#[test]
fn a_missing_or_foreign_file_is_unknown() {
    let d = dir("foreign");
    assert_eq!(session_state(&d.join("Cookies"), NOW), None);
    fs::write(d.join("Cookies"), "not a database").unwrap();
    assert_eq!(session_state(&d.join("Cookies"), NOW), None);
    let db = rusqlite::Connection::open(d.join("Other")).unwrap();
    db.execute_batch("CREATE TABLE t (x INTEGER)").unwrap();
    drop(db);
    assert_eq!(session_state(&d.join("Other"), NOW), None);
}

// The saved sign-ins on this computer, read only: cargo test real_profiles -- --ignored --nocapture
#[test]
#[ignore]
fn real_profiles() {
    let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).unwrap();
    let root = PathBuf::from(home).join(".claude-usage-dashboard").join("desktop").join("profiles");
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as i64;
    for e in fs::read_dir(root).unwrap().flatten() {
        let t = std::time::Instant::now();
        let s = session_state(&e.path().join("Cookies"), now);
        println!("{} {:?} in {:?}", e.file_name().to_string_lossy(), s, t.elapsed());
    }
    if let Some(app) = super::find_data_dir() {
        println!("Claude Desktop now: {:?}", session_state(&app.join("Network").join("Cookies"), now));
    }
}

#[test]
fn the_store_is_left_as_it_was() {
    let p = store("untouched", &[(".claude.ai", "sessionKey", 0)]);
    let before = fs::read(&p).unwrap();
    assert_eq!(session_state(&p, NOW), Some(true));
    assert_eq!(fs::read(&p).unwrap(), before);
    let siblings: Vec<_> = fs::read_dir(p.parent().unwrap()).unwrap().flatten().map(|e| e.file_name()).collect();
    assert_eq!(siblings.len(), 1, "no journal or temp file left beside it: {siblings:?}");
}
