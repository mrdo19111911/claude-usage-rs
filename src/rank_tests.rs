// Tests of the auto-switch ranking (docs/specs/auto-switch-deadline.md, "Test first").

use super::*;
use crate::usage_log::tests::account;

const NOW: i64 = 1_800_000_000_000;

fn settings(on: bool) -> Settings {
    let mut s: Settings =
        serde_json::from_str(r#"{"threshold":95,"autoSwitch":true,"pollSeconds":120}"#).unwrap();
    s.use_soonest_reset = on;
    s
}

// An account at five/week %, its weekly window resetting in `week_h` hours.
fn acc(id: &str, five: i64, week: i64, week_h: i64) -> Account {
    let mut a = account(id, five, week, vec![]);
    let u = a.usage.as_mut().unwrap();
    u.five.resets_at = Some(NOW + 2 * HOUR);
    u.week.resets_at = Some(NOW + week_h * HOUR);
    a
}

fn pick_id(v: &[Account], active: &str, on: bool) -> Option<String> {
    match pick(v, active, &settings(on), NOW) {
        Pick::Switch(id, _) => Some(id),
        _ => None,
    }
}

#[test]
fn old_settings_use_the_soonest_reset() {
    let s: Settings =
        serde_json::from_str(r#"{"threshold":95,"autoSwitch":true,"pollSeconds":120}"#).unwrap();
    assert!(s.use_soonest_reset);
}

#[test]
fn quota_about_to_expire_is_used_first() {
    let v = vec![acc("a", 5, 1, 131), acc("b", 0, 66, 8)];
    assert_eq!(pick_id(&v, "a", true).as_deref(), Some("b"));
    match pick(&v, "a", &settings(true), NOW) {
        Pick::Switch(_, why) => assert_eq!(why, "auto: weekly resets in 8h, sooner"),
        _ => unreachable!(),
    }
    assert_eq!(
        pick_id(&v, "a", false),
        None,
        "old rule: stay under the thresholds"
    );
}

#[test]
fn deadlines_under_an_hour_apart_do_not_switch() {
    let mut v = vec![acc("a", 5, 1, 10), acc("b", 0, 66, 10)];
    v[1].usage.as_mut().unwrap().week.resets_at = Some(NOW + 10 * HOUR - 30 * 60_000);
    assert_eq!(pick_id(&v, "a", true), None);
}

#[test]
fn the_same_reset_a_second_apart_ranks_by_room() {
    let (mut x, y) = (acc("x", 50, 60, 20), acc("y", 0, 60, 20));
    x.usage.as_mut().unwrap().week.resets_at = Some(NOW + 20 * HOUR + 1_500);
    let v = vec![x, y];
    let ids: Vec<&str> = order(&v, &settings(true), NOW).into_iter().map(|i| v[i].id.as_str()).collect();
    assert_eq!(ids, ["y", "x"], "more room first");
}

#[test]
fn a_sliver_of_five_hour_room_does_not_switch() {
    assert_eq!(
        pick_id(&[acc("a", 5, 1, 100), acc("b", 90, 50, 8)], "a", true),
        None
    );
    assert_eq!(
        pick_id(&[acc("a", 5, 1, 100), acc("b", 80, 50, 8)], "a", true).as_deref(),
        Some("b")
    );
}

#[test]
fn at_a_threshold_the_earliest_deadline_wins_not_the_most_room() {
    let v = vec![
        acc("a", 95, 10, 100),
        acc("b", 0, 1, 120),
        acc("c", 0, 60, 20),
    ];
    assert_eq!(pick_id(&v, "a", true).as_deref(), Some("c"));
    assert_eq!(
        pick_id(&v, "a", false).as_deref(),
        Some("b"),
        "old rule: most room"
    );
    match pick(&v, "a", &settings(true), NOW) {
        Pick::Switch(_, why) => assert_eq!(why, "auto: 5-hour 95%"),
        _ => unreachable!(),
    }
    // A useful account beats an earlier one with a sliver of 5-hour room.
    let v = vec![
        acc("a", 95, 10, 100),
        acc("b", 0, 1, 120),
        acc("c", 92, 60, 20),
    ];
    assert_eq!(pick_id(&v, "a", true).as_deref(), Some("b"));
}

#[test]
fn nothing_eligible_says_which_limit() {
    let v = vec![acc("a", 10, 99, 100), acc("b", 0, 100, 120)];
    assert!(matches!(pick(&v, "a", &settings(true), NOW), Pick::NoRoom(w) if w == "weekly 99%"));
}

#[test]
fn a_window_whose_reset_passed_counts_as_zero() {
    let mut b = acc("b", 0, 100, 1);
    b.usage.as_mut().unwrap().week.resets_at = Some(NOW - HOUR);
    let v = vec![acc("a", 96, 10, 100), b];
    assert_eq!(pick_id(&v, "a", true).as_deref(), Some("b"));
    // Its deadline is unknown: ranked after every known one.
    let v2 = vec![acc("a", 96, 10, 100), v[1].clone(), acc("c", 0, 50, 150)];
    assert_eq!(pick_id(&v2, "a", true).as_deref(), Some("c"));
}

#[test]
fn disabled_errored_or_unchecked_are_never_picked() {
    let (mut b, mut c, mut d) = (acc("b", 0, 50, 5), acc("c", 0, 50, 6), acc("d", 0, 50, 7));
    b.enabled = false;
    c.error = Some("expired".into());
    d.usage = None;
    assert!(matches!(
        pick(&[acc("a", 96, 10, 100), b, c, d], "a", &settings(true), NOW),
        Pick::NoRoom(_)
    ));
}

#[test]
fn auto_switch_off_never_switches() {
    let mut s = settings(true);
    s.auto_switch = false;
    assert!(matches!(
        pick(&[acc("a", 99, 10, 100), acc("b", 0, 50, 5)], "a", &s, NOW),
        Pick::Stay
    ));
}

#[test]
fn the_order_shown_follows_the_ranking() {
    let mut off = acc("off", 0, 0, 2);
    off.enabled = false;
    let mut full_week = acc("fw", 10, 100, 30);
    full_week.usage.as_mut().unwrap().week.resets_at = Some(NOW + 30 * HOUR);
    let full_five = acc("ff", 96, 40, 50); // usable again at its 5-hour reset, in 2 h
    let v = vec![
        off,
        acc("late", 5, 1, 131),
        full_week,
        acc("soon", 0, 66, 8),
        full_five,
    ];
    let ids: Vec<&str> = order(&v, &settings(true), NOW)
        .into_iter()
        .map(|i| v[i].id.as_str())
        .collect();
    assert_eq!(ids, ["soon", "late", "ff", "off", "fw"], "100% last, even after disabled ones");
    let saved: Vec<usize> = order(&v, &settings(false), NOW);
    assert_eq!(saved, [0, 1, 3, 4, 2], "setting off: the saved order, 100% last");
}

#[test]
fn accounts_at_100_go_last_soonest_free_first() {
    let mut late = acc("late", 100, 40, 50);
    late.usage.as_mut().unwrap().five.resets_at = Some(NOW + 4 * HOUR);
    let soon = acc("soon", 100, 40, 50); // 5-hour back in 2 h
    let mut gone = acc("gone", 100, 40, 50); // its 5-hour reset passed: 0%, not full
    gone.usage.as_mut().unwrap().five.resets_at = Some(NOW - HOUR);
    let v = vec![late, soon, gone];
    for on in [true, false] {
        let ids: Vec<&str> = order(&v, &settings(on), NOW).into_iter().map(|i| v[i].id.as_str()).collect();
        assert_eq!(ids, ["gone", "soon", "late"], "setting {on}");
    }
}
