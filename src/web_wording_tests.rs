// Plain wording for office staff (docs/REVIEW_DE_HIEU.md, the High rows of sections 3 and 4):
// "Claude Code" never "Claude CLI", "In use" not "Active", one check-frequency rule written the
// same on the dashboard and on the help page, no ratio between the two limits.

use super::asset;

fn text(path: &str) -> &'static str {
    asset(path).unwrap_or_else(|| panic!("{path} served")).0
}

// The check schedule as src/schedule.rs (next_check_at) does it, in the same words on both pages.
pub(super) const CHECK_RULE: &str = "Usage is checked every 10 minutes; an account in use is checked more often as it nears a limit, every minute from 90% of its 5-hour limit (or near its weekly limit). A used-up account is checked again after it resets.";

// The paragraph of `src` that holds `needle`.
fn para<'a>(src: &'a str, needle: &str) -> &'a str {
    let i = src.find(needle).unwrap_or_else(|| panic!("{needle} found"));
    let s = src[..i].rfind("<p").unwrap();
    &src[s..i + src[i..].find("</p>").unwrap()]
}

#[test]
fn the_pages_say_claude_code_never_claude_cli() {
    for p in ["/", "/help.html", "/js/sides.js", "/js/card.js", "/js/main.js"] {
        assert!(!text(p).contains("Claude CLI"), "{p} says Claude Code");
    }
    assert!(text("/help.html").contains("The <b>Code</b> and <b>Desktop</b> buttons"), "the widget's button is Code");
    let page = text("/");
    assert!(page.contains(">+ Add account for Claude Code</button>"));
    assert!(page.contains(">+ Add account for Claude Desktop</button>"));
    assert!(!page.contains("Add account ·"));
}

#[test]
fn the_accounts_note_is_three_short_sentences() {
    let page = text("/");
    for s in ["Claude Code (terminal and VS Code) switches at once. Claude Desktop restarts on the new account; any reply in progress is cut off.",
        "\"Allow auto-switch\" lets the app move to this account when the one in use is nearly used up.",
        "Charts show use from the start of each limit to its reset; the dotted line is steady use."] {
        assert!(page.contains(s), "the note says: {s}");
    }
    for old in ["Their <b>auto-switch</b> switches", "even pace", "dotted diagonal"] {
        assert!(!page.contains(old), "gone: {old}");
    }
}

#[test]
fn the_thresholds_read_as_one_sentence_and_keep_their_inputs() {
    let page = text("/");
    assert!(page.contains("Switch Claude Code when 5-hour use reaches <input type=\"number\" id=\"threshold\" min=\"50\" max=\"100\">%</label>
      <label>or weekly use reaches <input type=\"number\" id=\"weekThreshold\" min=\"50\" max=\"100\">%"));
    let at = crate::switch::desktop_auto::DESKTOP_AT;
    assert!(page.contains(&format!("Claude Desktop switches at {at}% of either (it restarts)")));
}

#[test]
fn the_soonest_reset_setting_is_plain() {
    assert!(text("/").contains("<input type=\"checkbox\" id=\"useSoonestReset\"> Use up first the account whose weekly limit resets soonest, so no weekly allowance goes to waste (may switch before a limit)</label>"));
}

#[test]
fn both_pages_state_the_same_check_rule() {
    for p in ["/", "/help.html"] {
        let t = text(p);
        assert_eq!(t.matches(CHECK_RULE).count(), 1, "{p} states the rule once");
        for old in ["at that pace", "eight times", "6.7%"] {
            assert!(!t.contains(old), "{p}: gone: {old}");
        }
    }
}

#[test]
fn the_dashboard_scripts_use_the_plain_names() {
    let rows = text("/js/rows.js");
    assert!(rows.contains("`${ready} accounts ready · weekly use left: about ${(head / 100).toFixed(1)} accounts' worth`"));
    assert!(!rows.contains("headroom"));
    let sides = text("/js/sides.js");
    assert!(sides.contains("` · 5-hour ${effWin(u.five).u}% · weekly ${effWin(u.week).u}%`"));
    let card = text("/js/card.js");
    assert!(card.contains("<span></span></span>allow auto-switch</label>"));
    let base = text("/js/base.js");
    assert!(base.contains("'In use']") && !base.contains("'Active'"));
    let main = text("/js/main.js");
    assert!(main.contains("<h2>App stopped</h2><p class=\"note\">Open Smartlog Claude Auto Switch from the Start menu to start it again.</p>"));
    assert!(!main.contains("scas.exe"));
}

#[test]
fn help_explains_paused_and_drops_the_ratio() {
    let h = text("/help.html");
    assert!(h.contains("What does \"Paused for a while\" mean?"));
    assert!(text("/js/tidy.js").contains("Paused for a while"), "the page really shows it");
    assert!(h.contains("Anthropic asked the app to check less often. It waits 5 minutes, then longer each time (up to 1 hour). The figures shown stay as they were."));
    assert!(!h.contains("HTTP 429") && !h.contains("five times as much"));
    assert!(h.contains("The weekly limit is much larger, so 1% of it is more use than 1% of the 5-hour limit."));
    assert!(h.contains("<b>Claude Code</b> is the Claude app in a terminal and in VS Code; they share one sign-in."));
}

#[test]
fn help_lists_the_auto_switch_rules_with_the_code_defaults() {
    let h = text("/help.html");
    let s = crate::model::State::default().settings;
    let at = crate::switch::desktop_auto::DESKTOP_AT;
    for line in [format!("<li>Claude Code: switches at {}% (5-hour) or {}% (weekly).</li>", s.threshold, s.week_threshold),
        format!("<li>Claude Desktop: switches at {at}%, only to accounts with \"allow auto-switch\" on, at most every 10 min; it restarts.</li>"),
        "<li>Accounts with \"allow auto-switch\" off are skipped.</li>".to_string()] {
        assert!(h.contains(&line), "help lists: {line}");
    }
    assert!(para(h, "resets soonest").contains("Weekly use you don't spend before the reset is lost, so the app uses first the account that resets soonest."));
    assert!(!h.contains("quota would expire") && !h.contains("the old rule"));
}
