// Tests of the 4:00 renewal of saved Claude Desktop sign-ins (docs/specs/desktop-renew.md).

use super::*;

const NOW: i64 = 1_800_000_000_000;
const DAY_MS: i64 = 86_400_000;
const TODAY: i64 = 20_271_112;

fn at(hour: u32) -> Clock {
    Clock { ms: NOW, hour, day: TODAY }
}

fn desk(ids: &[&str]) -> Vec<DesktopAccount> {
    ids.iter().map(|i| DesktopAccount { uuid: i.to_string(), label: i.to_string(), auto: false }).collect()
}

// "a" is signed in; "b" ends in 5 days, "c" in 3, "d" in 20; "x" has ended or is unknown.
fn until(u: &str) -> Option<i64> {
    match u {
        "a" => Some(NOW + 2 * DAY_MS),
        "b" => Some(NOW + 5 * DAY_MS),
        "c" => Some(NOW + 3 * DAY_MS),
        "d" => Some(NOW + 20 * DAY_MS),
        _ => None,
    }
}

fn step(st: &RenewState, signed_in: Option<&str>, code: bool, c: &Clock) -> Step {
    next_step(true, st, signed_in, &desk(&["a", "b", "c", "d", "x"]), &until, code, c)
}

#[test]
fn at_four_it_renews_the_saved_sign_in_that_ends_first() {
    assert_eq!(step(&RenewState::default(), Some("a"), false, &at(4)), Step::Renew("c".into()));
}

#[test]
fn only_at_four_only_when_on_and_only_with_somewhere_to_come_back_to() {
    let st = RenewState::default();
    for h in [0, 3, 5, 12, 23] {
        assert_eq!(step(&st, Some("a"), false, &at(h)), Step::Wait, "hour {h}");
    }
    assert_eq!(step(&st, None, false, &at(4)), Step::Wait, "Desktop signed out");
    let off = next_step(false, &st, Some("a"), &desk(&["a", "c"]), &until, false, &at(4));
    assert_eq!(off, Step::Wait, "setting off");
}

#[test]
fn far_ended_or_unknown_sign_ins_are_not_due() {
    let d = desk(&["a", "d", "x"]);
    assert_eq!(next_step(true, &RenewState::default(), Some("a"), &d, &until, false, &at(4)), Step::Wait);
}

#[test]
fn the_signed_in_account_is_never_renewed_by_switching() {
    let d = desk(&["a", "d"]);
    assert_eq!(next_step(true, &RenewState::default(), Some("a"), &d, &until, false, &at(4)), Step::Wait, "a ends in 2 days but is in use");
}

#[test]
fn a_running_code_session_waits_unless_the_sign_in_ends_within_two_days() {
    let st = RenewState::default();
    assert_eq!(step(&st, Some("a"), true, &at(4)), Step::Wait, "c ends in 3 days: next night");
    let soon = |u: &str| if u == "b" { Some(NOW + DAY_MS) } else { None };
    assert_eq!(next_step(true, &st, Some("a"), &desk(&["a", "b"]), &soon, true, &at(4)), Step::Renew("b".into()));
}

#[test]
fn one_tried_today_is_not_tried_again_until_tomorrow() {
    let st = RenewState { renewing: None, day: TODAY, tried: vec!["c".into()] };
    assert_eq!(step(&st, Some("a"), false, &at(4)), Step::Renew("b".into()));
    let both = RenewState { renewing: None, day: TODAY, tried: vec!["b".into(), "c".into()] };
    assert_eq!(step(&both, Some("a"), false, &at(4)), Step::Wait);
    let yesterday = RenewState { renewing: None, day: TODAY - 1, tried: vec!["b".into(), "c".into()] };
    assert_eq!(step(&yesterday, Some("a"), false, &at(4)), Step::Renew("c".into()));
}

#[test]
fn it_stays_three_minutes_then_goes_back_then_is_done() {
    let r = |at: i64| RenewState { renewing: Some(Renewing { target: "c".into(), back: "a".into(), at }), day: TODAY, tried: vec!["c".into()] };
    let c = at(4);
    assert_eq!(step(&r(NOW - STAY_MS + 1), Some("c"), false, &c), Step::Wait);
    assert_eq!(step(&r(NOW - STAY_MS), Some("c"), false, &c), Step::Back("a".into()));
    assert_eq!(step(&r(NOW - STAY_MS), Some("a"), false, &c), Step::Done);
    // Going back does not wait for 4:00, the setting or a quiet Desktop.
    let off = next_step(false, &r(NOW - STAY_MS), Some("c"), &desk(&["a", "c"]), &until, true, &at(9));
    assert_eq!(off, Step::Back("a".into()));
}

#[test]
fn the_state_survives_a_restart_in_state_json() {
    let st = RenewState { renewing: Some(Renewing { target: "c".into(), back: "a".into(), at: NOW }), day: TODAY, tried: vec!["c".into()] };
    let text = serde_json::to_string(&st).unwrap();
    assert!(text.contains("\"renewing\"") && text.contains("\"back\":\"a\""), "{text}");
    assert_eq!(serde_json::from_str::<RenewState>(&text).unwrap(), st);
    assert_eq!(serde_json::from_str::<RenewState>("{}").unwrap(), RenewState::default());
}

// The rule on this computer's saved sign-ins at 4:00 today and each night after, as if
// none were renewed: cargo test real_renew_nights -- --ignored --nocapture
#[test]
#[ignore]
fn real_renew_nights() {
    let home = std::path::PathBuf::from(std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).unwrap());
    let state: serde_json::Value = serde_json::from_slice(&std::fs::read(home.join(".claude-usage-dashboard/state.json")).unwrap()).unwrap();
    let desk: Vec<DesktopAccount> = serde_json::from_value(state["desktop"].clone()).unwrap();
    let store = desktop::Store { root: home.join(".claude-usage-dashboard/desktop") };
    let dir = desktop::find_data_dir().unwrap();
    let signed_in = desktop::signed_in(&dir);
    let now = now();
    let until = |u: &str| match store.saved_session(u, now) {
        Some(desktop::Session::Live(x)) => x,
        _ => None,
    };
    for d in &desk {
        println!("{} {}: until {:?} ({:.1} days)", &d.uuid[..8], d.label, until(&d.uuid), until(&d.uuid).map_or(f64::NAN, |u| (u - now) as f64 / DAY_MS as f64));
    }
    println!("signed in: {signed_in:?}, code running: {}", desktop::RealApp { data_dir: dir.clone() }.code_running());
    for n in 0..30 {
        let c = Clock { ms: now + n * DAY_MS, hour: 4, day: TODAY + n };
        let s = next_step(true, &RenewState::default(), signed_in.as_deref(), &desk, &until, false, &c);
        if s != Step::Wait {
            println!("night +{n}: {s:?}");
        }
    }
}
