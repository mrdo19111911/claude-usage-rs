// Sharing Claude Desktop's Code sessions on the page (docs/specs/desktop-sessions.md,
// "Page"): a Settings checkbox under Auto-switch, read from /api/state's
// `settings.shareDesktopSessions` (missing = on), sent like the other settings; one help answer.

use super::asset;

const LABEL: &str = "Show every account's Code sessions in Claude Desktop (when switching Desktop)";

fn page() -> &'static str {
    asset("/").unwrap().0
}

fn js(name: &str) -> &'static str {
    asset(&format!("/js/{name}")).expect("served").0
}

#[test]
fn the_settings_card_has_the_checkbox_under_auto_switch() {
    let p = page();
    let auto = p.find("id=\"autoSwitch\"").expect("auto-switch checkbox");
    let share = p.find("<input type=\"checkbox\" id=\"shareDesktopSessions\">").expect("a checkbox");
    assert!(auto < share, "under Auto-switch");
    assert!(share < p.find("id=\"updRow\"").unwrap(), "above Updates, in the Settings card");
    let label = &p[share..share + p[share..].find("</label>").unwrap()];
    assert!(label.contains(LABEL), "labelled: {label}");
    // Same look as the auto-switch checkbox: inside a .settings row.
    let row = p[..share].rfind("<div").unwrap();
    assert!(p[row..share].starts_with("<div class=\"settings\""), "in a .settings row");
}

#[test]
fn the_checkbox_shows_the_setting_and_missing_counts_as_on() {
    let js = js("settings.js");
    assert!(js.contains("$('#shareDesktopSessions').checked = s.shareDesktopSessions !== false"));
}

#[test]
fn a_change_is_sent_like_the_other_settings() {
    let js = js("main.js");
    let lp = js.find("for (const id of [").expect("the settings loop");
    let list = &js[lp..lp + js[lp..].find(']').unwrap()];
    assert!(list.contains("'shareDesktopSessions'"), "has its onchange: {list}");
    let body = &js[lp..lp + js[lp..].find("}) }));").unwrap()];
    assert!(body.contains("'/api/settings'"));
    assert!(body.contains("shareDesktopSessions: $('#shareDesktopSessions').checked"), "sent");
}

#[test]
fn help_says_switching_desktop_shares_the_sessions() {
    let h = asset("/help.html").unwrap().0;
    let q = h.find("Code sessions").expect("a Code sessions answer");
    let end = q + h[q..].find("</details>").unwrap();
    let a = &h[q..end];
    for s in ["copies", "same sessions", "never lost", "this computer", "first message", "quota", "while a reply is running"] {
        assert!(a.contains(s), "the answer says {s}");
    }
}
