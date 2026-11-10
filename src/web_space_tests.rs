// Less space in "In use now" and the panes (docs/specs/account-cards.md, "Less space"):
// one panel when both apps use one account, no bar under a chart, short notes.

use super::asset;

fn js(name: &str) -> &'static str {
    asset(&format!("/js/{name}")).unwrap_or_else(|| panic!("{name} served")).0
}

#[test]
fn both_apps_on_one_account_share_one_panel() {
    let s = js("sides.js");
    assert!(s.contains("const same = !!(cur && da && cur === da)"), "same account: the CLI login is Desktop's");
    assert!(s.contains("$('#sides').classList.toggle('both', same)"), "one panel across both columns");
    assert!(s.contains("function both("), "the shared panel");
    assert!(s.contains("Both on this account"));
    // Each app keeps its own note and switch menu, with its tag.
    assert!(s.contains("function control(kind, title, note, opts, busy)"));
    let css = asset("/style.css").unwrap().0;
    assert!(css.contains(".sides.both #sideDesk { display: none; }"));
    assert!(css.contains(".sides.both #sideCode { grid-column: 1 / -1;"));
    assert!(css.contains("linear-gradient(90deg, var(--code) 50%, var(--desk) 50%)"), "half CLI, half Desktop colour");
}

#[test]
fn a_pane_says_what_is_left_in_its_title_not_under_a_bar() {
    let c = js("card.js");
    let f = c.find("function pane(").expect("pane()");
    let body = &c[f..f + c[f..].find("
}").unwrap()];
    assert!(!body.contains("bar("), "no bar under the chart");
    assert!(body.contains("paneLeft(w)"), "the title row says what is left");
    let p = c.find("function paneLeft(w)").expect("paneLeft()");
    let pl = &c[p..p + c[p..].find("
}").unwrap()];
    assert!(pl.contains("color(w.u)") && pl.contains("% left") && pl.contains("timeLeft(w.resetsAt)") && pl.contains("not started"));
    // The countdown alone: the clock time is under the chart's axis. resetLabel keeps both.
    let b = asset("/js/base.js").unwrap().0;
    assert!(b.contains("function timeLeft(t)") && b.contains("return `in ${timeLeft(t)} (${day} ${hhmm(t)})`"));
}

#[test]
fn the_notes_are_one_short_line_with_the_rest_in_a_tooltip() {
    let s = js("sides.js");
    for short in ["`Auto-switch at ${s.threshold}% 5-hour · ${s.weekThreshold ?? 99}% weekly`", "'Auto-switch at 98% · switching restarts it'", "'Auto-switch is off'"] {
        assert!(s.contains(short), "short note {short}");
    }
    assert!(s.contains("<div class=\"note\" title=\"${esc(note[1])}\">${note[0]}</div>"), "full text in the tooltip");
}
