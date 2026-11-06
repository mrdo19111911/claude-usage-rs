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
    let body = &c[f..f + c[f..].find("\n}\n").unwrap()];
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
