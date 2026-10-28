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
    let css = attrs(page, "href");
    let js = attrs(page, "src");
    assert!(!css.is_empty() && !js.is_empty(), "the page loads its style and scripts from files");
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
fn the_accounts_are_two_lists_cli_left_desktop_right() {
    let page = asset("/").unwrap().0;
    let cli = page.find("id=\"cliRows\"").expect("a Claude CLI list");
    let desk = page.find("id=\"deskRows\"").expect("a Claude Desktop list");
    assert!(cli < desk, "Claude CLI comes first (left)");
    assert!(page.contains("id=\"accts\""), "one section holds both lists");
    assert!(!page.contains("<th>Claude Desktop</th>"), "the shared six-column table is gone");
    for js in ["js/list_cli.js", "js/list_desk.js"] {
        assert!(page.contains(&format!("src=\"{js}\"")), "{js} loaded");
    }
}
