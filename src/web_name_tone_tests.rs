// The account name's colour on its card (docs/specs/account-cards.md): red when a limit is
// reached, white under 80% weekly and 50% 5-hour, yellow otherwise.

use super::asset;

fn text(path: &str) -> &'static str {
    asset(path).unwrap_or_else(|| panic!("{path} served")).0
}

fn func(src: &'static str, name: &str) -> &'static str {
    let s = src.find(&format!("function {name}(")).unwrap_or_else(|| panic!("{name} defined"));
    let e = src[s + 9..].find("
function ").map_or(src.len(), |i| s + 9 + i);
    &src[s..e]
}

#[test]
fn the_tone_uses_both_limits_after_a_passed_reset() {
    let f = func(text("/js/base.js"), "nameTone");
    assert!(f.contains("effWin(a.usage.five)") && f.contains("effWin(a.usage.week)"), "{f}");
    assert!(f.contains(">= 100") && f.contains("'full'"), "red at a reached limit: {f}");
    assert!(f.contains("< 80") && f.contains("< 50"), "white under 80% weekly and 50% 5-hour: {f}");
    assert!(f.contains("'low'"), "yellow otherwise: {f}");
}

#[test]
fn the_card_name_carries_the_tone_and_css_colours_it() {
    let w = func(text("/js/card.js"), "who");
    assert!(w.contains("nameTone(a)") && w.contains("class=\"email"), "{w}");
    let css = text("/style.css");
    assert!(css.contains(".email.full") && css.contains("var(--red)"), "red");
    assert!(css.contains(".email.low") && css.contains("var(--amber)"), "yellow");
}
