// Tests of Claude Desktop's auto-switch (docs/specs/account-cards.md, "Claude Desktop
// auto-switch (backend)").

use super::*;
use crate::usage_log::tests::account;

const NOW: i64 = 1_800_000_000_000;
const MIN: i64 = 60_000;

fn settings() -> Settings {
    serde_json::from_str(r#"{"threshold":95,"autoSwitch":true,"pollSeconds":120}"#).unwrap()
}

// Account `id` (Desktop uuid "u-<id>") at five/week %, its weekly window resetting in `week_h` hours.
fn acc(id: &str, five: i64, week: i64, week_h: i64) -> Account {
    let mut a = account(id, five, week, vec![]);
    a.account_uuid = Some(format!("u-{id}"));
    let u = a.usage.as_mut().unwrap();
    u.five.resets_at = Some(NOW + 2 * HOUR);
    u.week.resets_at = Some(NOW + week_h * HOUR);
    a
}

fn desk(id: &str, auto: bool) -> DesktopAccount {
    DesktopAccount { uuid: format!("u-{id}"), label: id.into(), auto }
}

fn saved(_: &str) -> bool {
    true
}

// Desktop on "a" (5h 100%), with "b" (resets later) and "c" (resets sooner), all `auto`.
fn three() -> (Vec<Account>, Vec<DesktopAccount>) {
    let v = vec![acc("a", 100, 40, 50), acc("b", 0, 10, 120), acc("c", 0, 60, 20)];
    let d = vec![desk("a", true), desk("b", true), desk("c", true)];
    (v, d)
}

fn run(v: &[Account], d: &[DesktopAccount], s: &Settings, last: i64) -> Pick {
    pick_desktop(v, d, Some("u-a"), &saved, s, NOW, last)
}

fn target(p: Pick) -> Option<String> {
    match p {
        Pick::Switch(uuid, _) => Some(uuid),
        _ => None,
    }
}

#[test]
fn at_100_switches_to_the_first_ranked_with_the_reason() {
    let (v, d) = three();
    match run(&v, &d, &settings(), 0) {
        Pick::Switch(uuid, why) => {
            assert_eq!(uuid, "u-c", "earliest weekly reset first");
            assert_eq!(why, "Claude Desktop: auto-switch (5-hour 100%)");
        }
        _ => panic!("expected a switch"),
    }
    let mut v = v;
    v[0].usage.as_mut().unwrap().five.u = 20;
    v[0].usage.as_mut().unwrap().week.u = 100;
    assert!(matches!(run(&v, &d, &settings(), 0), Pick::Switch(_, w) if w == "Claude Desktop: auto-switch (weekly 100%)"));
}

#[test]
fn under_100_stays_even_past_the_cli_threshold() {
    let (mut v, d) = three();
    v[0].usage.as_mut().unwrap().five.u = 99;
    assert!(matches!(run(&v, &d, &settings(), 0), Pick::Stay));
    v[0].usage.as_mut().unwrap().week.u = 99;
    assert!(matches!(run(&v, &d, &settings(), 0), Pick::Stay));
}

#[test]
fn a_window_whose_reset_passed_is_not_at_100() {
    let (mut v, d) = three();
    v[0].usage.as_mut().unwrap().five.resets_at = Some(NOW - MIN);
    assert!(matches!(run(&v, &d, &settings(), 0), Pick::Stay));
}

#[test]
fn only_auto_on_with_a_saved_sign_in() {
    let (v, mut d) = three();
    d[2].auto = false;
    assert_eq!(target(run(&v, &d, &settings(), 0)).as_deref(), Some("u-b"));
    d[2].auto = true;
    let no_c = |u: &str| u != "u-c";
    assert_eq!(target(pick_desktop(&v, &d, Some("u-a"), &no_c, &settings(), NOW, 0)).as_deref(), Some("u-b"));
    // Not in the Desktop list at all: never signed in there.
    assert_eq!(target(run(&v, &d[..2], &settings(), 0)).as_deref(), Some("u-b"));
}

#[test]
fn only_useful_room_and_never_the_account_in_use() {
    let (mut v, d) = three();
    v[2].usage.as_mut().unwrap().five.u = 90; // a sliver of 5-hour room: skipped
    assert_eq!(target(run(&v, &d, &settings(), 0)).as_deref(), Some("u-b"));
    v[1].enabled = false; // not in auto-switch's ranking
    assert!(matches!(run(&v, &d, &settings(), 0), Pick::NoRoom(w) if w == "5-hour 100%"));
}

#[test]
fn no_candidate_says_which_limit() {
    let (mut v, d) = three();
    v[1].usage.as_mut().unwrap().week.u = 100;
    v[2].error = Some("expired".into());
    assert!(matches!(run(&v, &d, &settings(), 0), Pick::NoRoom(w) if w == "5-hour 100%"));
}

#[test]
fn ten_minutes_between_desktop_auto_switches() {
    let (v, d) = three();
    assert!(matches!(run(&v, &d, &settings(), NOW - 9 * MIN), Pick::Stay));
    assert!(matches!(run(&v, &d, &settings(), NOW - 10 * MIN), Pick::Switch(..)));
}

#[test]
fn auto_switch_off_never_switches() {
    let (v, d) = three();
    let mut s = settings();
    s.auto_switch = false;
    assert!(matches!(run(&v, &d, &s, 0), Pick::Stay));
}

#[test]
fn signed_out_or_unknown_usage_stays() {
    let (mut v, d) = three();
    assert!(matches!(pick_desktop(&v, &d, None, &saved, &settings(), NOW, 0), Pick::Stay));
    // Signed in to an account with no Claude CLI login here: no figures.
    assert!(matches!(pick_desktop(&v, &d, Some("u-z"), &saved, &settings(), NOW, 0), Pick::Stay));
    v[0].usage = None;
    assert!(matches!(run(&v, &d, &settings(), 0), Pick::Stay));
}

#[test]
fn old_saved_desktop_accounts_have_auto_off() {
    let d: DesktopAccount = serde_json::from_str(r#"{"uuid":"u-x","label":"x"}"#).unwrap();
    assert!(!d.auto);
}
