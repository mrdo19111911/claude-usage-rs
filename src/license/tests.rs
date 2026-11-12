// Tests for the license module (docs/specs/license.md, "Test").

use super::*;

const NOW: i64 = 1_800_000_000_000;

fn saved() -> Saved {
    Saved { trial_start: None, key: KeyState::None, checked_at: 0, ads: false }
}

#[test]
fn hash_ignores_case_and_spaces_and_hides_the_email() {
    let a = email_hash("Bob@Example.com ");
    assert_eq!(a, email_hash("bob@example.com"));
    assert_ne!(a, email_hash("alice@example.com"));
    assert_eq!(a.len(), 64, "hex SHA-256");
    assert!(!a.contains("bob"));
}

#[test]
fn the_trial_starts_at_the_earliest_email() {
    // Server answers per email hash: Some(start) when seen before, None when new.
    assert_eq!(trial_start(&[None, Some(NOW - 5 * DAY), Some(NOW - 2 * DAY)], NOW), NOW - 5 * DAY);
    // A new email added later does not restart it.
    assert_eq!(trial_start(&[Some(NOW - 20 * DAY), None], NOW), NOW - 20 * DAY);
    assert_eq!(trial_start(&[None, None], NOW), NOW, "all new: starts now");
}

#[test]
fn the_trial_lasts_thirty_days() {
    let s = Saved { trial_start: Some(NOW), ..saved() };
    assert_eq!(status(&s, NOW), Status::Trial { days_left: 30 });
    assert_eq!(status(&s, NOW + 7 * DAY), Status::Trial { days_left: 23 });
    assert_eq!(status(&s, NOW + 30 * DAY), Status::Expired { why: "trial over" });
    // Not asked yet (offline at the first run): the full trial.
    assert_eq!(status(&saved(), NOW), Status::Trial { days_left: 30 });
}

#[test]
fn a_paid_key_holds_seven_days_offline() {
    let s = Saved { trial_start: Some(NOW - 90 * DAY), key: KeyState::Active, checked_at: NOW - 3 * DAY, ..saved() };
    assert_eq!(status(&s, NOW), Status::Licensed);
    let s = Saved { checked_at: NOW - 8 * DAY, ..s };
    assert_eq!(status(&s, NOW), Status::Expired { why: "not checked for 7 days" });
}

#[test]
fn a_revoked_key_ends_it() {
    let s = Saved { key: KeyState::Revoked, checked_at: NOW, ..saved() };
    assert_eq!(status(&s, NOW), Status::Expired { why: "revoked" });
}

#[test]
fn ads_keep_it_going_after_the_trial() {
    let s = Saved { trial_start: Some(NOW - 40 * DAY), ads: true, ..saved() };
    assert_eq!(status(&s, NOW), Status::Ads);
}

#[test]
fn only_expired_stops_auto_switch() {
    assert!(allows_auto(&Status::Trial { days_left: 1 }));
    assert!(allows_auto(&Status::Licensed));
    assert!(allows_auto(&Status::Ads));
    assert!(!allows_auto(&Status::Expired { why: "trial over" }));
}

// Asked 2026-11-12: 1-2 accounts free; 3 or more pay; any account with "gosmartlog" free forever.
#[test]
fn one_or_two_accounts_are_free() {
    let over = Saved { trial_start: Some(NOW - 90 * DAY), ..saved() };
    assert_eq!(status_for(&over, &["a@x.com"], NOW), Status::Free { why: "1-2 accounts" });
    assert_eq!(status_for(&over, &["a@x.com", "b@x.com"], NOW), Status::Free { why: "1-2 accounts" });
    assert_eq!(status_for(&over, &["a@x.com", "b@x.com", "c@x.com"], NOW), Status::Expired { why: "trial over" });
    // The same email twice counts once.
    assert_eq!(status_for(&over, &["a@x.com", "A@x.com ", "b@x.com"], NOW), Status::Free { why: "1-2 accounts" });
}

#[test]
fn a_gosmartlog_account_is_free_forever() {
    let over = Saved { trial_start: Some(NOW - 900 * DAY), key: KeyState::Revoked, ..saved() };
    let l = ["a@x.com", "b@x.com", "hanh.do@GoSmartlog.com"];
    assert_eq!(status_for(&over, &l, NOW), Status::Free { why: "gosmartlog" });
    assert!(allows_auto(&Status::Free { why: "gosmartlog" }));
}
