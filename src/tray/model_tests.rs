// Tests for model.rs: state JSON -> widget model, time-left text.

use super::*;

#[test]
fn reads_both_sides_from_the_state() {
    let st = r#"{"now":1000,"settings":{"threshold":94},"activeId":"b","demo":false,
        "accounts":[{"id":"a","email":"a@x.com","usage":{"five":{"u":10,"resetsAt":2000},"week":{"u":100,"resetsAt":9000}}},
                    {"id":"b","email":"b@x.com","usage":null}],
        "desktop":{"found":true,"accounts":[{"label":"Account 1","email":null,"signedIn":true,"usage":null}]}}"#;
    let m = model(st);
    assert_eq!(m.cli.email.as_deref(), Some("b@x.com"));
    assert!(m.cli.usage.is_none());
    assert_eq!(m.desk.unwrap().email.as_deref(), Some("Account 1"));
    assert_eq!(m.others.len(), 1);
    assert_eq!(m.others[0].usage.as_ref().unwrap().week.u, 100);
}

// Accounts a, b, c (CLI on `active`); Desktop knows `desk` (uuid, hasSignIn, signedIn).
fn state(active: &str, desk: &[(&str, bool, bool)], found: bool, demo: bool, switching: bool) -> String {
    let desk: Vec<Value> = desk.iter().map(|(u, has, on)| json!({"uuid": u, "email": format!("{u}@x.com"), "label": u, "hasSignIn": has, "signedIn": on})).collect();
    let accts: Vec<Value> = ["a", "b", "c"].iter().map(|i| json!({"id": i, "uuid": format!("u{i}"), "email": format!("{i}@x.com"), "usage": null})).collect();
    json!({"now": 1, "activeId": active, "demo": demo, "accounts": accts,
           "desktop": {"found": found, "switching": switching, "accounts": desk}}).to_string()
}

fn other<'a>(m: &'a Model, id: &str) -> &'a Other {
    m.others.iter().find(|o| o.id == id).unwrap_or_else(|| panic!("{id} not in others"))
}

#[test]
fn others_carry_their_cli_id() {
    let m = model(&state("a", &[], true, false, false));
    let ids: Vec<&str> = m.others.iter().map(|o| o.id.as_str()).collect();
    assert_eq!(ids, ["b", "c"]);
    assert!(m.others.iter().all(|o| !o.in_cli));
}

#[test]
fn desk_uuid_only_with_a_saved_sign_in() {
    let m = model(&state("a", &[("ub", true, false), ("uc", false, false)], true, false, false));
    assert_eq!(other(&m, "b").desk_uuid.as_deref(), Some("ub"));
    assert_eq!(other(&m, "c").desk_uuid, None);
}

#[test]
fn in_desk_from_signed_in() {
    let m = model(&state("a", &[("ub", true, true), ("uc", true, false)], true, false, false));
    assert!(other(&m, "b").in_desk);
    assert!(!other(&m, "c").in_desk);
}

#[test]
fn cli_account_listed_only_when_desktop_is_on_another() {
    // Desktop on b: a (CLI) is listed too, for its Desktop button.
    let m = model(&state("a", &[("ua", true, false), ("ub", true, true)], true, false, false));
    assert_eq!(m.others.len(), 3);
    let a = other(&m, "a");
    assert!(a.in_cli && !a.in_desk);
    assert_eq!(a.desk_uuid.as_deref(), Some("ua"));
    assert_eq!(m.cli.email.as_deref(), Some("a@x.com"));
    // Desktop on a too: a only in the CLI section.
    let m = model(&state("a", &[("ua", true, true), ("ub", true, false)], true, false, false));
    assert_eq!(m.others.len(), 2);
    assert!(m.others.iter().all(|o| o.id != "a"));
}

#[test]
fn desk_switching_from_the_state() {
    assert!(model(&state("a", &[("ub", true, true)], true, false, true)).desk_switching);
    assert!(!model(&state("a", &[("ub", true, true)], true, false, false)).desk_switching);
}

#[test]
fn no_desk_data_when_not_found_or_demo() {
    for (found, demo) in [(false, false), (true, true)] {
        let m = model(&state("a", &[("ua", true, false), ("ub", true, true)], found, demo, true));
        assert!(m.desk.is_none() && !m.desk_switching);
        assert_eq!(m.others.len(), 2);
        assert!(m.others.iter().all(|o| o.desk_uuid.is_none() && !o.in_desk && !o.in_cli));
    }
}

#[test]
fn time_left() {
    assert_eq!(left(4 * 3_600_000 + 39 * 60_000), "4h39m");
    assert_eq!(left(51 * 3_600_000), "2d 3h");
    assert_eq!(left(30_000), "1m");
}

fn win(u: i64, reset: Option<i64>) -> Win {
    Win { u, reset }
}

fn uses(five: Win, week: Win) -> Option<Use> {
    Some(Use { five, week })
}

fn row(email: &str, usage: Option<Use>) -> Other {
    Other { email: email.into(), usage, ..Default::default() }
}

#[test]
fn pace_is_the_share_of_the_window_gone() {
    assert_eq!(pace(Some(1000 + FIVE_MS / 2), 1000, FIVE_MS), Some(0.5));
    assert_eq!(pace(Some(1000 + FIVE_MS * 2), 1000, FIVE_MS), Some(0.0));
    assert!((pace(Some(1000 + FIVE_MS / 4), 1000, FIVE_MS).unwrap() - 0.75).abs() < 1e-6);
}

#[test]
fn pace_none_without_a_future_reset() {
    assert_eq!(pace(None, 1000, FIVE_MS), None);
    assert_eq!(pace(Some(1000), 1000, FIVE_MS), None);
    assert_eq!(pace(Some(999), 1000, FIVE_MS), None);
}

#[test]
fn full_until_is_the_latest_full_reset() {
    let u = Use { five: win(100, Some(50)), week: win(100, Some(90)) };
    assert_eq!(full_until(&u), Some(90));
    // Only the full window counts.
    let u = Use { five: win(100, Some(50)), week: win(40, Some(90)) };
    assert_eq!(full_until(&u), Some(50));
    assert_eq!(full_until(&Use { five: win(99, Some(5)), week: win(0, None) }), None);
}

#[test]
fn full_until_max_when_a_full_window_has_no_reset() {
    let u = Use { five: win(100, None), week: win(100, Some(90)) };
    assert_eq!(full_until(&u), Some(i64::MAX));
}

#[test]
fn split_others_orders_both_lists() {
    let m = Model {
        others: vec![
            row("none@x", None),
            row("b@x", uses(win(30, None), win(45, None))),
            row("a@x", uses(win(50, None), win(10, None))),
            row("tie2@x", uses(win(50, None), win(40, None))),
            row("tie1@x", uses(win(50, None), win(40, None))),
            row("late@x", uses(win(100, Some(900)), win(5, None))),
            row("soon@x", uses(win(2, None), win(100, Some(300)))),
            row("same@x", uses(win(100, Some(300)), win(1, None))),
        ],
        ..Default::default()
    };
    let (open, full) = split_others(&m);
    let names: Vec<&str> = open.iter().map(|o| o.email.as_str()).collect();
    assert_eq!(names, ["b@x", "a@x", "tie1@x", "tie2@x", "none@x"]);
    let f: Vec<(&str, i64)> = full.iter().map(|(o, t)| (o.email.as_str(), *t)).collect();
    assert_eq!(f, [("same@x", 300), ("soon@x", 300), ("late@x", 900)]);
}

#[test]
fn same_account_needs_equal_emails_ignoring_case() {
    let side = |e: Option<&str>| Side { email: e.map(String::from), usage: None };
    let mut m = Model { cli: side(Some("A@X.com")), desk: Some(side(Some("a@x.COM"))), ..Default::default() };
    assert!(same_account(&m));
    m.desk = Some(side(Some("b@x.com")));
    assert!(!same_account(&m));
    m.desk = Some(side(None));
    assert!(!same_account(&m));
    m.desk = None;
    assert!(!same_account(&m));
    m.cli = side(None);
    m.desk = Some(side(None));
    assert!(!same_account(&m));
}

#[test]
fn checked_is_the_newest_fetch_of_any_account() {
    let st = r#"{"now":1,"accounts":[{"id":"a","usage":{"fetchedAt":500}},{"id":"b","usage":{"fetchedAt":900}},{"id":"c","usage":null}],
        "desktop":{"found":true,"accounts":[{"signedIn":true,"usage":{"fetchedAt":700}}]}}"#;
    assert_eq!(model(st).checked, Some(900));
    let st = r#"{"now":1,"accounts":[{"id":"a","usage":{"fetchedAt":500}}],"desktop":{"found":false,"accounts":[{"usage":{"fetchedAt":800}}]}}"#;
    assert_eq!(model(st).checked, Some(800));
    assert_eq!(model(&state("a", &[], true, false, false)).checked, None);
}
