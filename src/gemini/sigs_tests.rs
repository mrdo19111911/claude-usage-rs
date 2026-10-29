// Thought signatures come back by tool_use id, for 2 hours, at most 1000 of them.

use super::*;
use std::time::Duration;

#[test]
fn a_signature_comes_back_by_its_id() {
    let s = Sigs::default();
    s.put("toolu_a", "SIG_A");
    assert_eq!(s.get("toolu_a").as_deref(), Some("SIG_A"));
    assert_eq!(s.get("toolu_b"), None);
}

#[test]
fn empty_ids_and_signatures_are_not_kept() {
    let s = Sigs::default();
    s.put("", "SIG");
    s.put("toolu_a", "");
    assert_eq!(s.get(""), None);
    assert_eq!(s.get("toolu_a"), None);
}

#[test]
fn a_signature_is_gone_after_two_hours() {
    let s = Sigs::default();
    s.put("toolu_a", "SIG_A");
    let now = Instant::now();
    assert!(s.get_at("toolu_a", now + Duration::from_secs(2 * 3600 - 5)).is_some());
    assert!(s.get_at("toolu_a", now + Duration::from_secs(2 * 3600 + 5)).is_none());
}

#[test]
fn at_most_1000_are_kept_and_the_oldest_go_first() {
    let s = Sigs::default();
    for i in 0..1005 {
        s.put(&format!("toolu_{i}"), "SIG");
    }
    assert!(s.len() <= 1000);
    assert!(s.get("toolu_1004").is_some());
    assert!(s.get("toolu_0").is_none());
}
