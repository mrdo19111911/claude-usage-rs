// A saved Claude Desktop sign-in on its card: "Sign in again" once it has ended, and until
// when it lasts while it has not (docs/specs/desktop-signed-out.md).

use super::asset;

fn js(name: &str) -> &'static str {
    asset(&format!("/js/{name}")).unwrap_or_else(|| panic!("{name} served")).0
}

#[test]
fn an_expired_desktop_sign_in_asks_for_a_new_one() {
    // docs/specs/desktop-signed-out.md: no "Use here" or auto-switch toggle for it.
    let c = js("card.js");
    let f = c.find("function deskLogin(").unwrap();
    let body = &c[f..f + c[f..].find("\n}").unwrap()];
    let exp = body.find("d.signInExpired").expect("deskLogin reads signInExpired");
    assert!(exp < body.find("data-dswitch").unwrap(), "checked before Use here");
    assert!(body[exp..].contains("'Sign in again'") && body[exp..].contains("data-dfirst"));
    assert!(js("sides.js").contains("sign-in expired: sign in again"), "the In use now menu says it too");
}

#[test]
fn a_saved_desktop_sign_in_shows_until_when_it_lasts() {
    // docs/specs/desktop-signed-out.md, "Until when a saved sign-in lasts".
    let c = js("card.js");
    let f = c.find("function deskLogin(").unwrap();
    let body = &c[f..f + c[f..].find("\n}").unwrap()];
    assert!(body.contains("d.signInUntil"), "deskLogin reads signInUntil");
    let u = c.find("function untilLine(").expect("one function draws the line");
    let line = &c[u..u + c[u..].find("\n}").unwrap()];
    for part in ["Saved sign-in lasts until", "28 days", "Log out", "class=\"until"] {
        assert!(line.contains(part), "untilLine has {part}");
    }
    assert!(c[c.find("function login(").unwrap()..].contains("o.until"), "login() draws it");
    assert!(asset("/style.css").unwrap().0.contains(".until.soon"), "amber when it ends soon");
}

