// The 4:00 renewal on the page (docs/specs/desktop-renew.md, "Page"): a Settings checkbox
// under the Code sessions one, read from `settings.renewDesktop` (missing = on), sent like
// the other settings; the sign-in date's tooltip says it is renewed at 4:00.

use super::asset;

const LABEL: &str = "Keep saved Claude Desktop sign-ins from expiring (at 4:00 a.m. it switches Desktop briefly, then back)";

fn js(name: &str) -> &'static str {
    asset(&format!("/js/{name}")).expect("served").0
}

#[test]
fn the_settings_card_has_the_renew_checkbox_under_code_sessions() {
    let p = asset("/").unwrap().0;
    let share = p.find("id=\"shareDesktopSessions\"").expect("sessions checkbox");
    let renew = p.find("<input type=\"checkbox\" id=\"renewDesktop\">").expect("a checkbox");
    assert!(share < renew && renew < p.find("id=\"updRow\"").unwrap(), "under it, in Settings");
    let label = &p[renew..renew + p[renew..].find("</label>").unwrap()];
    assert!(label.contains(LABEL), "labelled: {label}");
}

#[test]
fn the_renew_checkbox_shows_the_setting_and_a_change_is_sent() {
    assert!(js("settings.js").contains("$('#renewDesktop').checked = s.renewDesktop !== false"));
    let m = js("main.js");
    let lp = m.find("for (const id of [").expect("the settings loop");
    let list = &m[lp..lp + m[lp..].find(']').unwrap()];
    assert!(list.contains("'renewDesktop'"), "has its onchange: {list}");
    let body = &m[lp..lp + m[lp..].find("}) }));").unwrap()];
    assert!(body.contains("renewDesktop: $('#renewDesktop').checked"), "sent");
}

#[test]
fn the_sign_in_date_says_it_is_renewed_at_four() {
    assert!(js("card.js").contains("renewed at 4:00"));
}
