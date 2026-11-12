// A reset that has passed (docs/specs/account-cards.md): the chart still marks it, the line
// drops to 0% at its time, and the status does not say "limit reached".

use super::asset;

fn text(path: &str) -> &'static str {
    asset(path).unwrap_or_else(|| panic!("{path} served")).0
}

// From `function {name}(` to the next top-level `function `.
fn func(src: &'static str, name: &str) -> &'static str {
    let s = src.find(&format!("function {name}(")).unwrap_or_else(|| panic!("{name} defined"));
    let e = src[s + 9..].find("\nfunction ").map_or(src.len(), |i| s + 9 + i);
    &src[s..e]
}

#[test]
fn a_passed_reset_is_kept_as_was() {
    let f = func(text("/js/base.js"), "effWin");
    assert!(f.contains("u: 0") && f.contains("resetsAt: null") && f.contains("was: w.resetsAt"), "{f}");
}

#[test]
fn the_status_counts_a_passed_reset_as_zero() {
    let f = func(text("/js/base.js"), "status");
    assert!(f.contains("effWin(a.usage.week)") && f.contains("effWin(a.usage.five)"), "{f}");
    assert!(!f.contains("= a.usage;"), "not the raw figures: {f}");
}

#[test]
fn the_switch_menu_counts_a_passed_reset_as_zero() {
    let f = func(text("/js/sides.js"), "usageHint");
    assert!(f.contains("effWin(u.five).u") && f.contains("effWin(u.week).u"), "{f}");
}

#[test]
fn the_chart_marks_a_passed_reset_and_drops_there() {
    let c = text("/js/chart.js");
    let pts = func(c, "winPoints");
    assert!(pts.contains("was"), "winPoints drops to 0 at the passed reset: {pts}");
    let ch = func(c, "winChart");
    assert!(ch.contains("w.was") && ch.contains("passed"), "winChart draws the passed reset: {ch}");
    assert!(ch.contains("var(--green)"));
}
