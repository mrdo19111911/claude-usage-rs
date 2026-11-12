// Wording fixes (docs/REVIEW_DE_HIEU.md, Medium and Low rows): tests written before the text changes.

use super::asset;

fn text(path: &str) -> &'static str {
    asset(path).unwrap_or_else(|| panic!("{path} served")).0
}

const SESSIONS: &str = "Keep the same Claude Code chat list in Claude Desktop on every account";
const RENEW: &str = "Keep saved Claude Desktop sign-ins from expiring (at 4:00 a.m. it switches Desktop briefly, then back)";

#[test]
fn settings_card_uses_plain_words() {
    let p = text("/");
    for w in [
        "Claude Code and Claude Desktop sign in separately: switching one never changes the other.",
        SESSIONS, RENEW,
        "<button class=\"btn\" id=\"refresh\">Check all now</button>",
        "<button class=\"btn\" id=\"import\">Look for a newly signed-in account</button>",
        ">Technical details for support (no passwords shown)</summary>",
        "Google may read what you send to Gemini,",
        "The account Claude Code uses now stays as it is.",
        "title=\"Close the app and take its icon out of the tray\"",
    ] {
        assert!(p.contains(w), "page says: {w}");
    }
    for old in ["Two separate logins", "Code sessions", "Renew saved", "Refresh now", "Check for new login",
                "no tokens shown", "send through", "The login Claude Code", "Stop the dashboard"] {
        assert!(!p.contains(old), "page: gone: {old}");
    }
}

#[test]
fn main_js_dialogs_use_plain_words() {
    let m = text("/js/main.js");
    for w in [
        "`Claude Desktop will restart signed in as ${d.email || d.label}. Any reply in progress is cut off. Continue?`",
        "'Remove this account from the app? (Your Claude account is not deleted.)'",
        "'Close Smartlog Claude Auto Switch? Usage checks and auto-switch stop until you open it again.'",
        "Then sign in there with ${ds.dfirst || 'the account to add'}. Continue?",
    ] {
        assert!(m.contains(w), "main.js says: {w}");
    }
    for old in ["Go on?", "Running tasks stop", "running tasks stop", "Continue with Google", "Stop the dashboard", "from the dashboard"] {
        assert!(!m.contains(old), "main.js: gone: {old}");
    }
}

#[test]
fn rows_js_says_sign_in_with_the_account() {
    let r = text("/js/rows.js");
    assert!(r.contains("Claude Desktop is open and signed out: sign in there with <b>${esc(D.expect)}</b>."));
    assert!(r.contains("'Claude Desktop is restarting (up to a minute)…'"));
    assert!(!r.contains("Continue with Google") && !r.contains("closing and will open again"));
}

#[test]
fn gemini_card_uses_plain_words() {
    let g = text("/js/gemini.js");
    assert!(g.contains("Quick tasks use: ${esc(g.fastModel)}"));
    assert!(g.contains("'Remove the Gemini access code from this computer? Claude Code on Gemini stops working until you add one again.'"));
    assert!(!g.contains("Small side jobs") && !g.contains("cgemini stops"));
}

#[test]
fn update_log_and_diagnostics_use_plain_words() {
    let u = text("/js/update.js");
    assert!(u.contains("`Install Smartlog Claude Auto Switch ${v} now? The app restarts in a moment.`"));
    assert!(!u.contains("dashboard closes"));
    let l = text("/js/usage_log.js");
    assert!(l.contains("n === 1 ? 'record' : 'records'") && !l.contains("'lines'"));
    let s = text("/js/settings.js");
    assert!(s.contains("`Last check: ") && !s.contains("Last poll"));
}
