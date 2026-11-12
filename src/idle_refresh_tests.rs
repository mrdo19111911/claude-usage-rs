// Tests of when a parked account's sign-in may be refreshed (docs/specs/idle-refresh.md).

use super::*;
use crate::usage_log::tests::account;

const T: i64 = 1_800_000_000_000;
const MIN: i64 = 60_000;

fn with(five: i64, week: i64, in_use: bool) -> Account {
    let mut a = account("a", 0, 0, vec![]);
    a.usage = Some(Usage { five: Win { u: five, resets_at: Some(T + HOUR) }, week: Win { u: week, resets_at: Some(T + 50 * HOUR) }, fetched_at: T, scoped: vec![] });
    a.in_use = in_use;
    a
}

#[test]
fn needed_only_when_an_account_in_use_is_near_a_limit() {
    assert!(!needed(&[with(50, 40, true)], T), "far from both limits");
    assert!(needed(&[with(80, 40, true)], T), "80% of the 5-hour limit");
    assert!(needed(&[with(10, 98, true)], T), "98% of the week is 86.6 five-hour points");
    assert!(needed(&[with(100, 40, true)], T), "a full window");
    assert!(!needed(&[with(95, 99, false)], T), "nobody in use: nothing to switch from");
}

#[test]
fn refresh_only_when_needed_or_asked_and_not_right_after_a_switch() {
    let long_ago = T - HOUR;
    assert!(!may_refresh(false, false, long_ago, T), "not needed");
    assert!(may_refresh(true, false, long_ago, T), "needed");
    assert!(may_refresh(false, true, long_ago, T), "Check now");
    assert!(!may_refresh(true, true, T - 5 * MIN, T), "left 5 min ago: an old window may hold it");
    assert!(may_refresh(true, false, T - LEFT_GUARD_MS, T), "the guard is over");
    assert!(may_refresh(true, false, 0, T), "never in use here");
}
