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
    assert!(s.contains("Both apps use this account"));
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
    assert!(pl.contains("color(w.u)") && pl.contains("% left") && pl.contains("timeLeft(w.resetsAt)") && pl.contains("no use yet"));
    // The countdown alone: the clock time is under the chart's axis. resetLabel keeps both.
    let b = asset("/js/base.js").unwrap().0;
    assert!(b.contains("function timeLeft(t)") && b.contains("return `in ${timeLeft(t)} (${day} ${hhmm(t)})`"));
}

#[test]
fn the_notes_are_one_short_line_with_the_rest_in_a_tooltip() {
    let s = js("sides.js");
    for short in ["`Auto-switch at ${s.threshold}% (5-hour) or ${s.weekThreshold ?? 99}% (weekly)`", "'Auto-switch at 98% · switching restarts the app'", "'Auto-switch is off'"] {
        assert!(s.contains(short), "short note {short}");
    }
    assert!(s.contains("<div class=\"note\" title=\"${esc(note[1])}\">${note[0]}</div>"), "full text in the tooltip");
}

#[test]
fn the_panels_charts_are_drawn_at_their_width_so_they_stay_low() {
    let c = js("chart.js");
    assert!(c.contains("function winChart(a, k, now, W = 300)"), "cards keep 300");
    let s = js("sides.js");
    assert!(s.contains("function paneWidth()"), "the pane's width in pixels");
    assert!(s.contains("const W = Math.max(300, paneWidth())"), "at least 300");
    assert!(s.contains("pane(a, 1, now, W)") && s.contains("pane(a, 2, now, W)"));
    let m = js("main.js");
    assert!(m.contains("new ResizeObserver(") && m.contains(".observe($('main'))"), "redrawn when the page's width changes");
    assert!(m.contains("if (w === drawnWidth) return;"), "not when only the height changes");
    assert!(m.contains("if (S) { renderSides(); renderCards(); }"), "once there is a state");
    assert!(js("card.js").contains("function pane(a, k, now, W)"));
}

#[test]
fn the_cards_charts_are_drawn_at_their_width_too() {
    let c = js("card.js");
    assert!(c.contains("function cardPaneWidth()"));
    assert!(c.contains("parseFloat(getComputedStyle($('#cards')).gridTemplateColumns)"), "the grid's column");
    assert!(c.contains("W = Math.max(300, cardPaneWidth())"), "at least 300");
    assert!(c.contains("card(a, d, now, W)") && c.contains("pane(a, 1, now, W)") && c.contains("pane(a, 2, now, W)"));
}

#[test]
fn the_model_chips_sit_in_the_weekly_title_row() {
    let c = js("card.js");
    let f = c.find("function pane(").unwrap();
    let body = &c[f..f + c[f..].find("
}").unwrap()];
    assert!(body.contains("<span class=\"pane-n\">${k === 1 ? '5-hour' : 'Weekly'}${k === 2 ? chips(a.usage) : ''}</span>"), "chips after Weekly");
    assert_eq!(body.matches("chips(").count(), 1, "not again under the chart");
    let css = asset("/style.css").unwrap().0;
    assert!(css.contains(".pane-t .scoped {"), "a chip sized for the title row");
}
