// The tray menu's pure parts: the help link and the "Check for updates" answer.

use super::*;

fn state(update: Value) -> String {
    json!({ "version": "1.1.0", "update": update }).to_string()
}

#[test]
fn help_link_is_next_to_the_dashboard() {
    assert_eq!(help_url("http://127.0.0.1:5000"), "http://127.0.0.1:5000/help.html");
    assert_eq!(help_url("http://127.0.0.1:5000/"), "http://127.0.0.1:5000/help.html");
}

#[test]
fn up_to_date_names_this_version() {
    let s = state(json!({ "status": "upToDate", "latest": "1.1.0", "url": null, "checkedAt": 1, "error": null }));
    assert_eq!(update_answer(&s, "1.1.0"), Answer::Info("Smartlog Claude Auto Switch 1.1.0 is the latest version.".into()));
}

#[test]
fn available_offers_to_install() {
    let s = state(json!({ "status": "available", "latest": "1.2.0", "url": "https://github.com/x", "checkedAt": 1, "error": null }));
    let want = "Smartlog Claude Auto Switch 1.2.0 is available (you have 1.1.0). Install now? The app closes and opens again.";
    assert_eq!(update_answer(&s, "1.1.0"), Answer::Offer(want.into()));
}

#[test]
fn available_without_a_version_cannot_be_offered() {
    let s = state(json!({ "status": "available", "latest": null }));
    assert_eq!(update_answer(&s, "1.1.0"), Answer::Fail("Could not check for updates.".into()));
}

#[test]
fn error_shows_its_text() {
    let s = state(json!({ "status": "error", "latest": null, "error": "GitHub: HTTP 403" }));
    assert_eq!(update_answer(&s, "1.1.0"), Answer::Fail("GitHub: HTTP 403".into()));
}

#[test]
fn error_without_text_says_it_could_not_check() {
    let s = state(json!({ "status": "error", "error": null }));
    assert_eq!(update_answer(&s, "1.1.0"), Answer::Fail("Could not check for updates.".into()));
}

#[test]
fn anything_else_could_not_check() {
    for st in ["idle", "checking", "downloading", "installing", "nonsense"] {
        let s = state(json!({ "status": st }));
        assert_eq!(update_answer(&s, "1.1.0"), Answer::Fail("Could not check for updates.".into()), "{st}");
    }
    assert_eq!(update_answer("{}", "1.1.0"), Answer::Fail("Could not check for updates.".into()));
    assert_eq!(update_answer("not json", "1.1.0"), Answer::Fail("Could not check for updates.".into()));
}

#[test]
fn menu_shows_the_crate_version() {
    assert_eq!(VERSION, env!("CARGO_PKG_VERSION"));
    assert_eq!(version_line(), format!("Smartlog Claude Auto Switch {}", env!("CARGO_PKG_VERSION")));
}
