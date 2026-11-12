// Tests of which pages stay in the dashboard window (docs/specs/dashboard-window.md).

use super::*;

#[test]
fn the_dashboard_own_pages_stay() {
    for u in ["http://127.0.0.1:4777", "http://127.0.0.1:4777/", "http://127.0.0.1:4777/help.html", "http://localhost:4777/?x=1", "http://127.0.0.1:4777#top"] {
        assert!(stays_inside(u, 4777), "{u}");
    }
}

#[test]
fn everything_else_goes_to_the_browser() {
    for u in [
        "http://127.0.0.1:4778/",
        "https://127.0.0.1:4777/",
        "https://claude.ai/",
        "about:blank",
        "http://127.0.0.1:4777.evil.com/",
        "http://127.0.0.1:47770/",
        "http://user@127.0.0.1:4777/",
    ] {
        assert!(!stays_inside(u, 4777), "{u}");
    }
}
