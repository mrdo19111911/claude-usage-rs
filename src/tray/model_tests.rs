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
