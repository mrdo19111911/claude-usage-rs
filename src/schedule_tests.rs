// Tests of the check schedule (docs/specs/check-schedule.md, "Tests first").

use super::*;
use crate::usage_log::tests::account;

const T: i64 = 1_800_000_000_000;
const MIN: i64 = 60_000;

fn at(five: i64, week: i64) -> Usage {
    Usage { five: Win { u: five, resets_at: Some(T + HOUR) }, week: Win { u: week, resets_at: Some(T + 50 * HOUR) }, fetched_at: T, scoped: vec![] }
}

#[test]
fn in_use_from_ten_minutes_at_zero_to_one_minute_from_ninety() {
    assert_eq!(next_check_at(&at(0, 0), true, T), T + 10 * MIN);
    assert_eq!(next_check_at(&at(45, 0), true, T), T + 11 * MIN / 2);
    assert_eq!(next_check_at(&at(90, 0), true, T), T + MIN);
    assert_eq!(next_check_at(&at(97, 0), true, T), T + MIN);
}

#[test]
fn the_week_counts_in_five_hour_points() {
    assert_eq!(next_check_at(&at(0, 72), true, T), T + 10 * MIN, "72% of a week is far from its limit");
    assert_eq!(next_check_at(&at(0, 97), true, T), T + 120_600, "97% of a week: 79.9 points, about 2 min");
    assert_eq!(next_check_at(&at(0, 99), true, T), T + MIN, "99% of a week: 93.3 points");
}

#[test]
fn not_in_use_every_ten_minutes() {
    for (f, w) in [(0, 0), (60, 10), (94, 98)] {
        assert_eq!(next_check_at(&at(f, w), false, T), T + 10 * MIN);
    }
}

#[test]
fn full_waits_for_its_reset() {
    for in_use in [true, false] {
        assert_eq!(next_check_at(&at(100, 40), in_use, T), T + HOUR + MIN);
        assert_eq!(next_check_at(&at(10, 100), in_use, T), T + 50 * HOUR + MIN);
    }
    let mut u = at(100, 40);
    u.five.resets_at = None;
    assert_eq!(next_check_at(&u, true, T), T + MIN, "reset unknown: soon");
    assert_eq!(next_check_at(&u, false, T), T + 10 * MIN);
    u.five.resets_at = Some(T - 1);
    assert_eq!(next_check_at(&u, true, T), T + MIN, "reset passed: soon");
}

// Four accounts last checked 9 min ago, next check in 5 min (old schedule).
fn four() -> Vec<Account> {
    let mut v: Vec<Account> = ["a", "b", "full", "err"].iter().map(|id| account(id, 20, 10, vec![])).collect();
    for a in v.iter_mut() {
        let u = a.usage.as_mut().unwrap();
        (u.fetched_at, u.five.resets_at, u.week.resets_at) = (T - 9 * MIN, Some(T + HOUR), Some(T + 50 * HOUR));
        a.next_check = T + 5 * MIN;
    }
    v[2].usage.as_mut().unwrap().five.u = 100;
    v[3].error = Some("offline".into());
    v[3].next_check = T - MIN + 2 * MIN;
    v
}

#[test]
fn a_change_of_use_works_the_next_check_out_again() {
    let mut v = four();
    v[0].in_use = true;
    mark_in_use(&mut v, Some("b"), None, T);
    assert!(!v[0].in_use && v[1].in_use);
    // 20% in use: 8 min after its last check (9 min ago), so due now.
    assert_eq!(v[1].next_check, T - MIN);
    assert_eq!(v[0].next_check, T + MIN, "no longer in use: 10 min after its last check");
}

#[test]
fn desktop_signed_in_counts_as_in_use() {
    let mut v = four();
    v[1].account_uuid = Some("uuid-b".into());
    mark_in_use(&mut v, None, Some("uuid-b"), T);
    assert!(v[1].in_use && !v[0].in_use);
}

#[test]
fn full_and_errored_accounts_keep_their_wait() {
    let mut v = four();
    let full_reset = v[2].usage.as_ref().unwrap().five.resets_at.unwrap();
    let err_next = v[3].next_check;
    mark_in_use(&mut v, Some("full"), None, T);
    assert_eq!(v[2].next_check, full_reset + MIN, "at 100%: still its reset");
    mark_in_use(&mut v, Some("err"), None, T);
    assert_eq!(v[3].next_check, err_next, "an error keeps its retry time");
    v[0].slow_until = T + 30 * MIN;
    mark_in_use(&mut v, Some("a"), None, T);
    assert_eq!(v[0].next_check, T + 30 * MIN, "after a 429: not before its wait");
}
