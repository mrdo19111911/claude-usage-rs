// Telling the normal and the demo instance's tray windows apart.

use super::*;

#[test]
fn normal_and_demo_classes_differ() {
    assert_eq!(tray_class(false), "cud-tray");
    assert_eq!(tray_class(true), "cud-tray-demo");
}

#[test]
fn demo_flag_read_from_state() {
    assert!(is_demo(r#"{"demo":true,"accounts":[]}"#));
    assert!(!is_demo(r#"{"demo":false,"accounts":[]}"#));
}

#[test]
fn missing_or_bad_state_is_not_demo() {
    assert!(!is_demo("{}"));
    assert!(!is_demo(""));
    assert!(!is_demo("not json"));
    // Only the top-level flag counts.
    assert!(!is_demo(r#"{"accounts":[{"demo":true}]}"#));
}

#[test]
fn quit_with_none_running_is_done_at_once() {
    // A class no window has: nothing to ask, nothing to wait for.
    let t = std::time::Instant::now();
    assert!(quit_class("cud-tray-test-none"));
    assert!(t.elapsed() < std::time::Duration::from_secs(1));
}
