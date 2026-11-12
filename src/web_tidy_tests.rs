// A tidy page (docs/specs/clean-page.md): the raw account error, the Settings card and the
// troubleshooting parts only show on a click; the glass stylesheet.

use super::asset;

fn page() -> &'static str {
    asset("/").unwrap().0
}

fn text(path: &str) -> &'static str {
    asset(path).unwrap_or_else(|| panic!("{path} served")).0
}

// The opening tag holding `id="{id}"`.
fn tag<'a>(p: &'a str, id: &str) -> &'a str {
    let i = p.find(&format!("id=\"{id}\"")).unwrap_or_else(|| panic!("#{id} on the page"));
    let s = p[..i].rfind('<').unwrap();
    &p[s..i + p[i..].find('>').unwrap()]
}

// From `<{name} … id="{id}"` to its closing tag (no nesting of the same tag inside).
fn block<'a>(p: &'a str, name: &str, id: &str) -> &'a str {
    let i = p.find(&format!("id=\"{id}\"")).unwrap();
    let s = p[..i].rfind(&format!("<{name}")).unwrap();
    &p[s..i + p[i..].find(&format!("</{name}>")).unwrap()]
}

#[test]
fn settings_open_from_a_button_in_the_top_bar() {
    let p = page();
    let bar = &p[p.find("class=\"topbar\"").unwrap()..p.find("</header>").unwrap()];
    assert!(bar.contains("id=\"settingsBtn\""), "the button is in the top bar");
    let b = tag(p, "settingsBtn");
    assert!(b.contains("aria-controls=\"settingsCard\"") && b.contains("aria-expanded=\"false\""), "{b}");
    let card = tag(p, "settingsCard");
    assert!(card.starts_with("<section") && card.contains(" hidden"), "hidden at first: {card}");
    let t = text("/js/tidy.js");
    assert!(t.contains("#settingsBtn") && t.contains("#settingsCard") && t.contains("scrollIntoView"));
}

#[test]
fn troubleshooting_parts_are_under_advanced() {
    let p = page();
    let settings = block(p, "section", "settingsCard");
    let adv = block(settings, "details", "advanced");
    assert!(!tag(p, "advanced").contains(" open"), "closed at first");
    assert!(adv.contains("<summary") && adv.contains("Advanced"));
    for id in ["refresh", "import", "logRow", "events", "diag", "logExport"] {
        assert!(adv.contains(&format!("id=\"{id}\"")), "#{id} under Advanced");
    }
    for id in ["exit", "updRow", "autoSwitch", "threshold"] {
        let s = format!("id=\"{id}\"");
        assert!(settings.contains(&s) && !adv.contains(&s), "#{id} in Settings, not under Advanced");
    }
}

#[test]
fn an_account_error_is_a_short_label_until_clicked() {
    let c = text("/js/card.js");
    assert!(!c.contains("esc(a.error)"), "the raw error is no longer drawn by card.js");
    assert!(c.contains("errBadge(a)"));
    let t = text("/js/tidy.js");
    for s in ["function errorLabel(", "function errBadge(", "Sign in again", "Can't check now", "/login",
        "Paused for a while", "429", "Waiting for Claude Code", "Anthropic is busy", "No connection",
        "data-signin", "data-errmore", "aria-expanded", "hidden", "errOpen"] {
        assert!(t.contains(s), "tidy.js has {s}");
    }
    // Plain words only: the raw text is never put on the page by tidy.js.
    for raw in ["esc(a.error)", "${a.error}", "esc(text)", "${text}"] {
        assert!(!t.contains(raw), "tidy.js does not draw the raw error ({raw})");
    }
    let p = page();
    let (tidy, card) = (p.find("src=\"js/tidy.js\"").expect("tidy.js loaded"), p.find("src=\"js/card.js\"").unwrap());
    assert!(tidy < card, "tidy.js before card.js");
}

#[test]
fn the_glass_stylesheet_follows_the_base_one() {
    let g = text("/glass.css");
    for s in ["backdrop-filter", "@supports not", "prefers-reduced-transparency"] {
        assert!(g.contains(s), "glass.css has {s}");
    }
    for path in ["/", "/help.html"] {
        let p = text(path);
        let (base, glass) = (p.find("href=\"style.css\"").unwrap(), p.find("href=\"glass.css\"").expect("glass.css linked"));
        assert!(base < glass, "{path}: glass.css after style.css");
    }
}

// "cho màu đen sâu hơn": the page near black, the glass a deep tint of it.
#[test]
fn the_black_is_deep() {
    let g = text("/glass.css");
    let hex = |name: &str| -> u32 {
        let i = g.find(&format!("{name}: #")).unwrap_or_else(|| panic!("{name} set")) + name.len() + 3;
        u32::from_str_radix(&g[i..i + 6], 16).unwrap()
    };
    let luma = |c: u32| ((c >> 16) + (c >> 8 & 0xff) + (c & 0xff)) / 3;
    assert!(luma(hex("--bg")) <= 10, "page background near black");
    assert!(luma(hex("--glass-solid")) <= 22, "solid card a deep black");
}
