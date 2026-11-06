// One card per account (docs/specs/account-cards.md): the "Usage over time" section is
// gone; each card holds its two logins as equal peers, its 5-hour and weekly charts over
// their windows, and its bars.

use super::asset;

fn page() -> &'static str {
    asset("/").unwrap().0
}

fn js(name: &str) -> &'static str {
    asset(&format!("/js/{name}")).unwrap_or_else(|| panic!("{name} served")).0
}

#[test]
fn one_section_of_cards_replaces_the_chart_and_the_two_lists() {
    let p = page();
    let accts = p.find("id=\"accts\"").expect("the Claude accounts section");
    assert!(p.find("id=\"cards\"").expect("the cards") > accts);
    for gone in ["Usage over time", "id=\"chart\"", "id=\"range\"", "id=\"cliRows\"", "id=\"deskRows\""] {
        assert!(!p.contains(gone), "{gone} is gone");
    }
    for gone in ["list_cli.js", "list_desk.js"] {
        assert!(!p.contains(gone), "{gone} not loaded");
        assert!(asset(&format!("/js/{gone}")).is_none(), "{gone} not served");
    }
    let (chart, card, main) = (p.find("js/chart.js").unwrap(), p.find("js/card.js").unwrap(), p.find("js/main.js").unwrap());
    assert!(chart < card && card < main, "chart.js, then card.js, then main.js");
}

#[test]
fn the_two_logins_are_equal_peers() {
    let c = js("card.js");
    // One function draws both, so they share size, layout and parts.
    let f = c.find("function login(").expect("one login() for both");
    let calls = c.matches("login('code'").count() + c.matches("login('desk'").count();
    assert_eq!(calls, 2, "called once for Claude CLI, once for Claude Desktop");
    let body = &c[f..f + c[f..].find("\n}").unwrap()];
    for part in ["class=\"login ", "class=\"switch\"", "✓ In use"] {
        assert!(body.contains(part), "login() draws {part}");
    }
    let css = asset("/style.css").unwrap().0;
    assert!(css.contains(".logins { display: grid; grid-template-columns: 1fr 1fr"), "two equal columns");
}

#[test]
fn every_account_action_is_on_the_card() {
    let c = js("card.js");
    for a in ["data-activate", "data-signin", "data-dswitch", "data-dfirst", "data-toggle", "data-dauto", "data-remove", "data-dremove", "d.auto"] {
        assert!(c.contains(a), "card.js has {a}");
    }
    let m = js("main.js");
    assert!(m.contains("e.target.dataset.dauto"), "the Desktop switch is wired");
    assert!(m.contains("/api/desktop/${uuid}/auto"), "it posts to the Desktop auto route");
}

#[test]
fn each_chart_spans_its_window_up_to_the_reset() {
    let c = js("chart.js");
    for s in ["5 * H", "7 * 24 * H", "resetsAt - span", "getDate()", "even pace", "now", "Recording since", "weekThreshold"] {
        assert!(c.contains(s), "chart.js has {s}");
    }
}

#[test]
fn a_window_whose_reset_passed_shows_zero() {
    let b = asset("/js/base.js").unwrap().0;
    assert!(b.contains("function effWin(w)") && b.contains("w = effWin(w);"), "bars use the effective %");
    assert!(js("chart.js").contains("effWin(week ? a.usage.week : a.usage.five)"), "charts too");
}

#[test]
fn cards_sit_two_to_a_row_on_a_wide_screen() {
    let css = asset("/style.css").unwrap().0;
    assert!(css.contains("#cards { display: grid; grid-template-columns: repeat(auto-fill, minmax(min(100%, 500px), 1fr))"), "two columns, one on a phone");
    // A chart half as wide keeps its text readable: a smaller drawing, not a scaled one.
    assert!(js("chart.js").contains("const W = 300, Ht = 104"), "compact chart");
}

#[test]
fn ticks_have_faint_columns_and_now_is_a_red_dashed_line() {
    let c = js("chart.js");
    assert!(c.contains("y1=\"${T}\" y2=\"${y(0)}\" stroke=\"var(--grid)\""), "a faint column at each tick");
    assert!(c.contains("stroke=\"var(--red)\" stroke-dasharray=\"4 3\""), "now: red, dashed");
}

#[test]
fn plan_status_and_checks_sit_on_the_right_of_the_email() {
    let c = js("card.js");
    let w = c.find("function who(").unwrap();
    let body = &c[w..w + c[w..].find("\n}").unwrap()];
    let (l, r) = (body.find("class=\"who-l\"").expect("left: email"), body.find("class=\"who-r\"").expect("right: plan, state, checks"));
    assert!(l < r && body[r..].contains("checkLine(a)") && body[r..].contains("a.plan"));
    assert!(asset("/style.css").unwrap().0.contains(".who-r { text-align: right;"));
}

#[test]
fn the_reset_is_a_green_line_with_its_time_under_the_axis() {
    let c = js("chart.js");
    // A known reset: a green line at the window's end, "reset" on top, its time below.
    assert!(c.contains("const rx = x(end)"), "the reset sits at the window's end");
    assert!(c.contains("x1=\"${rx}\" x2=\"${rx}\" y1=\"${T - 4}\" y2=\"${y(0)}\" stroke=\"var(--green)\" stroke-width=\"2\""), "a green line");
    assert!(c.contains(">reset</text>"), "labelled reset");
    assert!(c.contains("const rlab = week ? `${day(end)} ${hhmm(end)}` : hhmm(end)"), "its time under the axis");
    // A tick label that would sit under the reset's time is left out (its column stays).
    assert!(c.contains("x(t) < rx - rw"), "no tick label under the reset's");
}

#[test]
fn cards_at_100_go_last_after_desktop_only_ones() {
    let r = asset("/js/rows.js").unwrap().0;
    assert!(r.contains("some(w => effWin(w).u >= 100)"), "100% of either limit, a passed reset as 0");
    assert!(r.contains("rows.filter(r => !full(r)).concat(rows.filter(full))"), "full ones last, order kept");
}
