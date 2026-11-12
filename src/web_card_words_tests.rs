// Wording fixes (docs/REVIEW_DE_HIEU.md, Medium and Low rows): tests written before the text changes.

use super::asset;

fn text(path: &str) -> &'static str {
    asset(path).unwrap_or_else(|| panic!("{path} served")).0
}

fn body(src: &'static str, sig: &str) -> &'static str {
    let f = src.find(sig).unwrap_or_else(|| panic!("{sig} found"));
    &src[f..f + src[f..].find("\n}").unwrap()]
}

fn has_none(src: &str, olds: &[&str]) {
    for old in olds {
        assert!(!src.contains(old), "gone: {old}");
    }
}

#[test]
fn base_status_uses_the_shared_names() {
    let b = text("/js/base.js");
    let st = body(b, "function status(a)");
    for s in ["'Usage not checked yet'", "'Checking usage…'", "'Skipped by auto-switch'"] {
        assert!(st.contains(s), "status says {s}");
    }
    assert!(st.contains("effWin(a.usage.five)"), "status still uses effWin");
    assert!(body(b, "function bar(").contains("' · no use yet'"));
    has_none(b, &["No figures yet", "'Waiting'", "'Off'", "not started"]);
}

#[test]
fn card_buttons_and_notes_use_the_shared_names() {
    let c = text("/js/card.js");
    let cli = body(c, "function cliLogin(");
    assert!(cli.contains("'not signed in yet'") && cli.contains("label: 'Switch to this'"));
    let desk = body(c, "function deskLogin(");
    for s in ["'Claude Desktop not installed'", "label: 'Sign in here'", "'available after first sign-in'", "label: 'Switch to this'"] {
        assert!(desk.contains(s), "deskLogin says {s}");
    }
    assert!(body(c, "function untilLine(").contains("Saved sign-in lasts until ${date} (${days} day"));
    assert!(body(c, "function who(").contains("title=\"Remove from this app\""));
    assert!(body(c, "function paneLeft(").contains("'no use yet'"));
    assert!(body(c, "function card(").contains("Usage not checked yet"));
    has_none(c, &["'Use here", "first time)", "no login yet", "Not on this computer", "Sign-in until",
        "title=\"Remove\"", "not started", "Waiting for figures"]);
}

#[test]
fn in_use_panels_say_whose_sign_in_and_what_restarts() {
    let s = text("/js/sides.js");
    for w in ["' · auto-switch off'", "`Auto-switch at ${s.threshold}% (5-hour) or ${s.weekThreshold ?? 99}% (weekly)`",
        "'Signs in separately'", "'Auto-switch at 98% · switching restarts the app'", "Both apps use this account"] {
        assert!(s.contains(w), "sides.js says {w}");
    }
    assert_eq!(s.matches("'Signs in separately'").count(), 2, "both Desktop panels");
    has_none(s, &["' · off'", "Its own sign-in", "restarts it'", "Both on this account", "running tasks stop"]);
}

#[test]
fn chart_words_are_plain() {
    let c = text("/js/chart.js");
    for w in ["If you keep this pace (last 20 min):", "`used up at ${hhmm(f.te)} (in ${inm})`",
        "`about ${Math.round(f.ue)}% used when it resets (${hhmm(f.end)})`", ">steady use<", "Tracked since ${hhmm(first[0])}",
        ">Usage not checked yet<"] {
        assert!(c.contains(w), "chart.js says {w}");
    }
    has_none(c, &["At the last 20 min's pace", "Expected limit", "Expected ~", ">even pace<", "Recording since", "No figures yet"]);
}
