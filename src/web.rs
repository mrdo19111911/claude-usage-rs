// The dashboard page and its files, built into the program.

const HTML: &str = "text/html; charset=utf-8";
const CSS: &str = "text/css; charset=utf-8";
const JS: &str = "text/javascript; charset=utf-8";

// The file served at `path` ("/" is the page): its text and content type.
pub fn asset(path: &str) -> Option<(&'static str, &'static str)> {
    match path {
        "/" | "/index.html" => Some((include_str!("web/index.html"), HTML)),
        "/help.html" => Some((include_str!("web/help.html"), HTML)),
        "/style.css" => Some((include_str!("web/style.css"), CSS)),
        "/glass.css" => Some((include_str!("web/glass.css"), CSS)),
        "/js/base.js" => Some((include_str!("web/js/base.js"), JS)),
        "/js/rows.js" => Some((include_str!("web/js/rows.js"), JS)),
        "/js/sides.js" => Some((include_str!("web/js/sides.js"), JS)),
        "/js/chart.js" => Some((include_str!("web/js/chart.js"), JS)),
        "/js/tidy.js" => Some((include_str!("web/js/tidy.js"), JS)),
        "/js/card.js" => Some((include_str!("web/js/card.js"), JS)),
        "/js/settings.js" => Some((include_str!("web/js/settings.js"), JS)),
        "/js/update.js" => Some((include_str!("web/js/update.js"), JS)),
        "/js/usage_log.js" => Some((include_str!("web/js/usage_log.js"), JS)),
        "/js/gemini.js" => Some((include_str!("web/js/gemini.js"), JS)),
        "/js/gemini_dialog.js" => Some((include_str!("web/js/gemini_dialog.js"), JS)),
        "/js/main.js" => Some((include_str!("web/js/main.js"), JS)),
        _ => None,
    }
}

// The pictures (PNG): the app icon for the tab (tools/make_icon.py), the Gemini dialog's steps.
pub fn image(path: &str) -> Option<&'static [u8]> {
    match path {
        "/img/app-icon.png" => Some(include_bytes!("web/img/app-icon.png")),
        "/img/gem-1-signin.png" => Some(include_bytes!("web/img/gem-1-signin.png")),
        "/img/gem-2-create.png" => Some(include_bytes!("web/img/gem-2-create.png")),
        "/img/gem-3-project.png" => Some(include_bytes!("web/img/gem-3-project.png")),
        "/img/gem-4-newproject.png" => Some(include_bytes!("web/img/gem-4-newproject.png")),
        "/img/gem-5-createkey.png" => Some(include_bytes!("web/img/gem-5-createkey.png")),
        "/img/gem-6-copy.png" => Some(include_bytes!("web/img/gem-6-copy.png")),
        _ => None,
    }
}
