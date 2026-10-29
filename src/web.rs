// The dashboard page and its files, built into the program.

const HTML: &str = "text/html; charset=utf-8";
const CSS: &str = "text/css; charset=utf-8";
const JS: &str = "text/javascript; charset=utf-8";

// The file served at `path` ("/" is the page): its text and content type.
pub fn asset(path: &str) -> Option<(&'static str, &'static str)> {
    match path {
        "/" | "/index.html" => Some((include_str!("web/index.html"), HTML)),
        "/style.css" => Some((include_str!("web/style.css"), CSS)),
        "/js/base.js" => Some((include_str!("web/js/base.js"), JS)),
        "/js/rows.js" => Some((include_str!("web/js/rows.js"), JS)),
        "/js/list_cli.js" => Some((include_str!("web/js/list_cli.js"), JS)),
        "/js/list_desk.js" => Some((include_str!("web/js/list_desk.js"), JS)),
        "/js/sides.js" => Some((include_str!("web/js/sides.js"), JS)),
        "/js/chart.js" => Some((include_str!("web/js/chart.js"), JS)),
        "/js/settings.js" => Some((include_str!("web/js/settings.js"), JS)),
        "/js/main.js" => Some((include_str!("web/js/main.js"), JS)),
        _ => None,
    }
}
