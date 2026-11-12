// Tests of the switch wording (spec at the top of switch_text.rs).

use super::*;

fn settings() -> Settings {
    serde_json::from_str(r#"{"threshold":95,"autoSwitch":true,"pollSeconds":120}"#).unwrap()
}

fn at(five: i64, week: i64) -> Usage {
    Usage { five: Win { u: five, resets_at: None }, week: Win { u: week, resets_at: None }, fetched_at: 0, scoped: vec![] }
}

#[test]
fn a_switch_names_the_app_and_both_accounts() {
    assert_eq!(switched(Some("a@x"), "b@x", BY_YOU), "Claude Code: switched from a@x to b@x (by you)");
    assert_eq!(switched(None, "b@x", BY_YOU), "Claude Code: switched to b@x (by you)");
}

#[test]
fn the_limit_reason_is_plain() {
    let s = settings();
    assert_eq!(limit_reason(&at(95, 10), &s).as_deref(), Some("5-hour limit at 95%"));
    assert_eq!(limit_reason(&at(100, 10), &s).as_deref(), Some("5-hour limit used up"));
    assert_eq!(limit_reason(&at(10, 99), &s).as_deref(), Some("weekly limit at 99%"));
    assert_eq!(limit_reason(&at(10, 100), &s).as_deref(), Some("weekly limit used up"));
    assert_eq!(limit_reason(&at(10, 96), &s), None);
}

#[test]
fn limit_reason_fires_exactly_when_switch_reason_does() {
    let s = settings();
    for f in [0, 94, 95, 100] {
        for w in [0, 98, 99, 100] {
            let u = at(f, w);
            assert_eq!(limit_reason(&u, &s).is_some(), crate::switch::switch_reason(&u, &s).is_some(), "{f}/{w}");
        }
    }
}

#[test]
fn the_sooner_reason_reads_on_its_own() {
    assert_eq!(sooner("b@x", 30), "b@x's weekly limit resets sooner, in 30 h, so it is used first");
}

#[test]
fn activate_logs_the_plain_line() {
    use crate::usage_log::tests::{account, test_app};
    let dir = std::env::temp_dir().join(format!("scas-switch-text-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut app = test_app(&dir);
    app.demo = true;
    *app.st.lock().unwrap() = State { accounts: vec![account("a", 1, 1, vec![]), account("b", 1, 1, vec![])], active_id: Some("a".into()), ..Default::default() };
    app.activate("b", BY_YOU).unwrap();
    let st = app.st.lock().unwrap();
    assert_eq!(st.events[0].msg, "Claude Code: switched from a@example.com to b@example.com (by you)");
    drop(st);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn no_room_says_the_limit_and_that_none_is_left() {
    assert_eq!(no_room("a@x", "5-hour 95%"), "a@x is at 95% of its 5-hour limit, and no other account has any left");
    assert_eq!(no_room("a@x", "weekly 100%"), "a@x has used up its weekly limit, and no other account has any left");
    assert_eq!(no_room(DESKTOP_ACCOUNT, "5-hour 98%"), "Claude Desktop's account is at 98% of its 5-hour limit, and no other account has any left");
}

#[test]
fn desktop_switching_is_not_clipped() {
    assert_eq!(desktop_switching("5-hour", 98), "Claude Desktop: switching automatically (5-hour limit at 98%)");
    assert_eq!(desktop_switching("weekly", 100), "Claude Desktop: switching automatically (weekly limit used up)");
}

#[test]
fn the_chat_list_lines_are_plain() {
    assert_eq!(chats_copied(3, 1, 0), "Claude Desktop: Claude Code chat list copied (3 new, 1 updated)");
    assert_eq!(chats_copied(0, 0, 2), "Claude Desktop: Claude Code chat list copied (0 new, 0 updated, 2 removed)");
    assert_eq!(CHATS_NOT_COPIED, "Claude Desktop: the Claude Code chat list could not be copied. It tries again at the next switch.");
}

#[test]
fn switch_refusals_say_what_to_do() {
    assert_eq!(OPENED_BY_CLAUDE, "This app was opened by Claude, so restarting Claude Desktop would close it too. Open it from the Start menu instead.");
    assert_eq!(STILL_READING, "Still reading which account Claude Code uses; try again in a few seconds");
    assert_eq!(DESKTOP_SIGNED_OUT, "Claude Desktop is signed out: sign in there with the account to add");
    assert_eq!(NOT_FOUND, "This account is no longer in the list. Reload the page.");
    assert_eq!(SOMETHING_WRONG, "Something went wrong; reload the page.");
    assert_eq!(NAME_NEEDED, "Type a name for this account.");
}

#[test]
fn a_failed_check_is_plain_and_hides_the_raw_text() {
    assert_eq!(check_failed(&ApiErr::Other("profile HTTP 500".into())), "Could not reach Anthropic to check this account, so it was not switched. Try again shortly.");
    let auth = ApiErr::Auth("refresh failed (400): invalid_grant — /login this account again".into());
    assert_eq!(check_failed(&auth), "Anthropic did not accept this account's sign-in, so it was not switched. Sign in to it again.");
    assert_eq!(check_failed(&ApiErr::Slow(None)), SLOWED);
    assert_eq!(SLOWED, "Anthropic asked the app to check less often, so it was not switched. Try again in a few minutes.");
    assert_eq!(OTHER_ACCOUNT, "This account's saved sign-in now belongs to a different account. Sign in to it again.");
}

#[test]
fn auto_switch_waits_on_a_429() {
    assert_eq!(auto_failed(SLOWED), "Auto-switch waited: Anthropic asked the app to check less often");
    assert_eq!(auto_failed("x"), "Auto-switch failed: x");
}

#[test]
fn an_account_that_is_gone_is_said_plainly() {
    use crate::usage_log::tests::test_app;
    let dir = std::env::temp_dir().join(format!("scas-switch-gone-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut app = test_app(&dir);
    app.demo = true;
    assert_eq!(app.activate("nobody", BY_YOU).unwrap_err(), NOT_FOUND);
    assert_eq!(app.set_desktop_auto("nobody", true).unwrap_err(), NOT_FOUND);
    let _ = std::fs::remove_dir_all(&dir);
}
