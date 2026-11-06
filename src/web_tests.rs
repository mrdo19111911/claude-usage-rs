// The page and every file it loads are served, each with its content type.

use super::asset;

const HTML: &str = "text/html; charset=utf-8";
const CSS: &str = "text/css; charset=utf-8";
const JS: &str = "text/javascript; charset=utf-8";

// The values of `attr="..."` in the page, in order.
fn attrs<'a>(page: &'a str, attr: &str) -> Vec<&'a str> {
    let key = format!(" {attr}=\"");
    page.match_indices(&key)
        .map(|(i, _)| {
            let rest = &page[i + key.len()..];
            &rest[..rest.find('"').unwrap()]
        })
        .collect()
}

#[test]
fn serves_the_page() {
    let (page, ctype) = asset("/").unwrap();
    assert!(page.contains("<title>Claude Usage</title>"));
    assert_eq!(ctype, HTML);
    assert_eq!(asset("/index.html").unwrap(), (page, HTML));
    assert!(asset("/nope").is_none());
}

#[test]
fn serves_every_file_the_page_loads() {
    let page = asset("/").unwrap().0;
    // Links to other sites (Google AI Studio) are not ours to serve.
    let local = attrs(page, "href").into_iter().filter(|h| !h.starts_with("https://"));
    let (css, links): (Vec<_>, Vec<_>) = local.partition(|h| h.ends_with(".css"));
    let (pics, js): (Vec<_>, Vec<_>) = attrs(page, "src").into_iter().partition(|s| s.ends_with(".png"));
    assert!(!css.is_empty() && !js.is_empty(), "the page loads its style and scripts from files");
    for src in pics {
        assert!(super::image(&format!("/{src}")).is_some(), "{src} not served");
    }
    for href in links {
        assert_eq!(asset(&format!("/{href}")).unwrap_or_else(|| panic!("{href} not served")).1, HTML, "{href}");
    }
    for href in css {
        let (text, ctype) = asset(&format!("/{href}")).unwrap_or_else(|| panic!("{href} not served"));
        assert_eq!(ctype, CSS, "{href}");
        assert!(!text.trim().is_empty(), "{href} is empty");
    }
    for src in js {
        let (text, ctype) = asset(&format!("/{src}")).unwrap_or_else(|| panic!("{src} not served"));
        assert_eq!(ctype, JS, "{src}");
        assert!(!text.trim().is_empty(), "{src} is empty");
    }
}

#[test]
fn the_page_has_no_inline_style_or_script() {
    let page = asset("/").unwrap().0;
    assert!(!page.contains("<style>"));
    assert!(!page.contains("<script>"));
}

#[test]
fn both_logins_have_an_add_button() {
    let page = asset("/").unwrap().0;
    for id in ["cliAdd", "deskAdd"] {
        assert!(page.contains(&format!("id=\"{id}\"")), "{id} in the page");
    }
    let main = asset("/js/main.js").unwrap().0;
    assert!(main.contains("$('#deskAdd').onclick"), "deskAdd wired");
    assert!(main.contains("dfirst: ''"), "deskAdd runs the Desktop first-time sign-in with no email");
}

#[test]
fn a_new_account_chart_shows_its_figures_now() {
    let chart = asset("/js/chart.js").unwrap().0;
    assert!(chart.contains("Recording since"), "says why the line is short");
    assert!(!chart.contains("Not enough figures in this range yet"), "figures are never hidden");
}

#[test]
fn not_started_windows_and_model_limits_are_shown() {
    let base = asset("/js/base.js").unwrap().0;
    assert!(base.contains("not started yet"));
    let rows = asset("/js/rows.js").unwrap().0;
    assert!(rows.contains("u.scoped") && rows.contains("class=\"scoped\""), "a chip per model limit");
}

#[test]
fn help_page_is_served_and_linked() {
    let (help, ctype) = asset("/help.html").expect("help.html served");
    assert_eq!(ctype, HTML);
    assert!(help.contains("<title>Claude Usage — Help</title>"));
    assert!(help.contains("href=\"style.css\""), "same style as the dashboard");
    assert!(help.contains("href=\"/\""), "a way back to the dashboard");
    assert!(help.matches("<details").count() >= 10, "one <details> per topic");
    for w in ["5-hour", "Weekly", "not started yet", "Fable", "Claude CLI", "Claude Desktop",
              "Auto-switch", "+ Add account", "429", "widget", "update", "uninstall", ".claude-usage-dashboard"] {
        assert!(help.to_lowercase().contains(&w.to_lowercase()), "help covers {w}");
    }
    let page = asset("/").unwrap().0;
    assert!(page.contains("href=\"help.html\""), "the top bar links Help");
}

#[test]
fn the_top_bar_shows_the_version_and_update() {
    let page = asset("/").unwrap().0;
    let top = page.find("class=\"topbar\"").expect("a top bar");
    assert!(top < page.find("class=\"card\"").unwrap(), "above the first card");
    for id in ["version", "updPill", "autoUpdate", "updCheck", "updInstall", "updStatus", "updRow"] {
        assert!(page.contains(&format!("id=\"{id}\"")), "{id} in the page");
    }
    assert!(page.contains("src=\"js/update.js\""), "update.js loaded");
    assert!(page.find("js/update.js").unwrap() < page.find("js/main.js").unwrap(), "before main.js");
    let js = asset("/js/update.js").expect("update.js served").0;
    for s in ["function renderUpdate", "/api/update/check", "/api/update/install", "autoUpdate", "S.version"] {
        assert!(js.contains(s), "update.js has {s}");
    }
    assert!(asset("/js/main.js").unwrap().0.contains("renderUpdate()"), "drawn with the rest");
}

#[test]
fn settings_have_a_weekly_threshold() {
    let page = asset("/").unwrap().0;
    assert!(page.contains("id=\"weekThreshold\""));
    let main = asset("/js/main.js").unwrap().0;
    assert!(main.contains("weekThreshold: +$('#weekThreshold').value"));
    let chart = asset("/js/chart.js").unwrap().0;
    assert!(chart.contains("weekThreshold"), "the weekly line uses its own threshold");
}
