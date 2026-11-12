// Auto-switch's "soonest reset first" on the page (docs/specs/auto-switch-deadline.md,
// "Page"): a Settings checkbox under Auto-switch, missing = on, sent like the other
// settings; the help answer says what it does.

use super::asset;

fn page() -> &'static str {
    asset("/").unwrap().0
}

fn js(name: &str) -> &'static str {
    asset(&format!("/js/{name}")).expect("served").0
}

#[test]
fn the_settings_card_has_the_checkbox_under_auto_switch() {
    let p = page();
    let auto = p.find("id=\"autoSwitch\"").expect("auto-switch checkbox");
    let it = p.find("<input type=\"checkbox\" id=\"useSoonestReset\">").expect("a checkbox");
    assert!(auto < it && it < p.find("id=\"updRow\"").unwrap(), "under Auto-switch, in Settings");
    let label = &p[it..it + p[it..].find("</label>").unwrap()];
    assert!(label.contains("resets soonest"), "labelled: {label}");
}

#[test]
fn shown_missing_as_on_and_sent_like_the_others() {
    assert!(js("settings.js").contains("$('#useSoonestReset').checked = s.useSoonestReset !== false"));
    let js = js("main.js");
    let lp = js.find("for (const id of [").expect("the settings loop");
    assert!(js[lp..lp + js[lp..].find(']').unwrap()].contains("'useSoonestReset'"));
    let body = &js[lp..lp + js[lp..].find("}) }));").unwrap()];
    assert!(body.contains("useSoonestReset: $('#useSoonestReset').checked"), "sent");
}

#[test]
fn help_says_the_soonest_reset_is_used_first() {
    let h = asset("/help.html").unwrap().0;
    let q = h.find("What is auto-switch?").expect("the auto-switch answer");
    let a = &h[q..q + h[q..].find("</details>").unwrap()];
    for s in ["resets soonest", "lost", "in that order", "Settings"] {
        assert!(a.contains(s), "the answer says {s}");
    }
}
