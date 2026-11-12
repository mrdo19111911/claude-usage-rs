// Loading state.json without losing it (docs/specs/state-load.md).

use super::*;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

fn dir(tag: &str) -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let d = std::env::temp_dir().join(format!("cud-sload-{tag}-{}-{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn no_file_is_a_first_run() {
    let p = dir("none").join("state.json");
    assert!(matches!(load(&p, 1), Ok(Loaded::Fresh)));
}

#[test]
fn a_saved_state_reads_back() {
    let p = dir("ok").join("state.json");
    let mut st = State::default();
    st.active_id = Some("abc".into());
    fs::write(&p, serde_json::to_string(&st).unwrap()).unwrap();
    match load(&p, 1) {
        Ok(Loaded::Read(s)) => assert_eq!(s.active_id.as_deref(), Some("abc")),
        _ => panic!("read"),
    }
}

// 2026-11-12: a null where a number belongs wiped 6 accounts; now the file is kept.
#[test]
fn a_file_that_does_not_read_is_kept_aside() {
    let d = dir("bad");
    let p = d.join("state.json");
    let bad = r#"{"accounts":[{"id":"x","slowUntil":null}]}"#;
    fs::write(&p, bad).unwrap();
    match load(&p, 1793000000000) {
        Ok(Loaded::Kept { aside, why }) => {
            assert_eq!(aside, d.join("state.json.unreadable-1793000000000"));
            assert_eq!(fs::read_to_string(&aside).unwrap(), bad, "original bytes kept");
            assert!(!why.is_empty());
        }
        _ => panic!("kept"),
    }
    assert!(!p.exists(), "nothing left at state.json for a save to write over");
    assert!(matches!(load(&p, 2), Ok(Loaded::Fresh)));
}

#[test]
fn a_file_that_cannot_be_read_stops_the_start() {
    let p = dir("folder").join("state.json");
    fs::create_dir_all(p.join("inside")).unwrap();
    assert!(load(&p, 1).is_err());
    assert!(p.join("inside").is_dir(), "left untouched");
}

// docs/REVIEW_DE_HIEU.md section 5: plain words, the file named for support.
#[test]
fn the_kept_note_is_plain_and_names_the_file() {
    let aside = Path::new(r"C:\Users\A\.claude-usage-dashboard\state.json.unreadable-7");
    assert_eq!(
        kept_note(aside),
        r"Your saved accounts could not be loaded. A copy was kept; please contact support (file: C:\Users\A\.claude-usage-dashboard\state.json.unreadable-7)."
    );
}
