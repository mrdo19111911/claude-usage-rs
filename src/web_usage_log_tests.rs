// The usage log on the page (docs/specs/usage-log.md, "Page"): a row in the Settings card
// under "Updates" with the line count and an Export link, drawn by usage_log.js from
// /api/state's `usageLog`; one help answer on what is recorded and what Export leaves out.

use super::asset;

fn page() -> &'static str {
    asset("/").unwrap().0
}

fn js() -> &'static str {
    asset("/js/usage_log.js").expect("usage_log.js served").0
}

#[test]
fn the_settings_card_has_a_usage_log_row_under_updates() {
    let p = page();
    let row = p.find("id=\"logRow\"").expect("a usage log row");
    assert!(p.find("id=\"updRow\"").unwrap() < row, "under Updates");
    assert!(row < p.find("id=\"events\"").unwrap(), "inside the Settings card");
    let tail = &p[row..row + p[row..].find("</div>").unwrap()];
    assert!(tail.contains("<b>Usage log</b>"), "named Usage log");
    for id in ["logStatus", "logExport"] {
        assert!(tail.contains(&format!("id=\"{id}\"")), "{id} in the row");
    }
    assert!(tail.contains("hidden"), "hidden until the state says what to show");
}

#[test]
fn usage_log_js_is_loaded_and_drawn_with_the_rest() {
    let p = page();
    let i = p.find("src=\"js/usage_log.js\"").expect("usage_log.js loaded");
    assert!(i < p.find("src=\"js/main.js\"").unwrap(), "before main.js");
    assert!(js().contains("function renderUsageLog"));
    assert!(asset("/js/main.js").unwrap().0.contains("renderUsageLog()"), "drawn with the rest");
}

#[test]
fn the_row_shows_the_lines_since_and_an_export_link() {
    let js = js();
    for s in ["S.usageLog", "lines", "since", "/api/usage-log/export", "download", "toLocaleString"] {
        assert!(js.contains(s), "usage_log.js has {s}");
    }
    // "28 Oct": day then short month.
    assert!(js.contains("'en-GB'") && js.contains("month: 'short'"), "a short date");
}

#[test]
fn no_line_yet_says_nothing_yet_and_has_no_button() {
    let js = js();
    assert!(js.contains("nothing yet"));
    assert!(js.contains("$('#logExport').hidden = !n"), "no Export with 0 lines");
}

#[test]
fn hidden_in_the_demo_and_on_an_older_backend() {
    let js = js();
    assert!(js.contains("S.demo"), "hidden in the demo");
    assert!(js.contains("$('#logRow').hidden = !log || !!S.demo"), "no row without usageLog");
}

#[test]
fn help_explains_the_usage_log_and_its_export() {
    let help = asset("/help.html").unwrap().0;
    let i = help.find("<summary>What is the usage log?").expect("a help topic");
    let topic = &help[i..i + help[i..].find("</details>").unwrap()];
    for w in ["5-hour", "weekly", "reset", "every account", "no conversation", "no email",
              "usage-log.jsonl", ".claude-usage-dashboard", "<b>Export</b>", "acc1", "acc2"] {
        assert!(topic.contains(w), "the topic covers {w}");
    }
}
