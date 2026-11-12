// Wording fixes (docs/REVIEW_DE_HIEU.md, Medium and Low rows): tests written before the text changes.

use super::asset;

fn text(path: &str) -> &'static str {
    asset(path).unwrap_or_else(|| panic!("{path} served")).0
}

fn help() -> &'static str {
    text("/help.html")
}

#[test]
fn help_says_period_not_window() {
    let h = help();
    assert!(h.contains("how much of the 5-hour (or weekly) period has passed"), "M row 1");
    assert!(!h.contains("window's time") && !h.contains("the window allows"), "no 'window' for a limit");
    assert!(!h.contains("<b>5-hour</b> window"), "the 5-hour limit is a limit");
}

#[test]
fn help_says_no_use_yet() {
    let h = help();
    assert!(h.contains("<b>No use yet</b>"), "names list: no use yet");
    assert!(!h.to_lowercase().contains("not started yet"));
}

#[test]
fn help_drops_rebuilds_its_context() {
    let h = help();
    assert!(h.contains("The first message after a switch uses a little more of the limit than usual."));
    assert!(!h.contains("rebuilds its context") && !h.contains("quota"));
}

#[test]
fn help_names_the_app_not_a_side() {
    let h = help();
    assert!(h.contains("switch Claude Code (or Claude Desktop) to that account"));
    assert!(!h.contains("that side"));
}

#[test]
fn help_lists_the_gemini_steps() {
    let h = help();
    let i = h.find("Claude Code on Gemini (cgemini)").unwrap();
    let topic = &h[i..i + h[i..].find("</details>").unwrap()];
    let ol = topic.find("<ol>").expect("the steps are a list");
    assert_eq!(topic[ol..topic.find("</ol>").unwrap()].matches("<li>").count(), 4, "four steps");
    assert!(topic.contains("Then press <b>Use this</b>"));
}

#[test]
fn help_has_no_rate_limit_or_flash_jargon() {
    let h = help();
    assert!(h.contains("Claude Code says it hit a limit: wait and try again."));
    assert!(h.contains("Only models with \"Flash\" in the name are free"));
    assert!(!h.contains("rate limit") && !h.contains("Only Flash models"));
}

#[test]
fn help_uses_the_menu_and_button_names() {
    let h = help();
    assert!(h.contains("<b>Keep widget on screen</b>") && !h.contains("Pin widget"));
    assert!(h.contains("<b>Check all now</b>") && !h.contains("Refresh now"));
}

#[test]
fn help_low_rows() {
    let h = help();
    assert!(h.contains("checks for a new version") && !h.contains("GitHub releases"));
    assert!(h.contains("Your sign-ins are only ever sent to Anthropic.") && !h.contains("tokens"));
    assert!(h.contains("says how much is <b>left</b>") && !h.contains("% left"));
    assert!(!h.contains("login"), "names list: sign-in, no login");
}
