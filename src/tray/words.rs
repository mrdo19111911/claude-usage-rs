// Every widget and tray string an office user reads, in one place so each can be tested
// (docs/REVIEW_DE_HIEU.md, sections 1-2 and "Names to use everywhere"). The menu's
// update answers stay in menu.rs; the header's "· 3 min ago" in layout.rs.

use super::*;

// The program's name: window, message box and tooltip titles.
pub(super) const APP: &str = "Smartlog Claude Auto Switch";
// Over the in-use card(s), as on the dashboard.
pub(super) const IN_USE: &str = "IN USE NOW";
// Over the accounts still usable. The review's "AVAILABLE · SWITCH TO" does not fit next
// to FIGS_KEY at 300 px, so it stays short.
pub(super) const SWITCH_TO: &str = "SWITCH TO";
// Over the "Switch to" figures: which is which.
pub(super) const FIGS_KEY: &str = "5-hour · weekly";
// Accounts that used up a limit, and the time each can be used again.
pub(super) const USED_UP: &str = "USED UP · AVAILABLE AT";
// A card or chart for an account whose usage has not been fetched.
pub(super) const NOT_CHECKED: &str = "Usage not checked yet";
// A limit with no reset time: nothing used in it yet.
pub(super) const NO_USE: &str = "no use yet";
// The tray menu's pin and exit items.
pub(super) const PIN_ITEM: &str = "Keep widget on screen";
pub(super) const EXIT_ITEM: &str = "Exit (stops auto-switch)";
// Exit asked from the menu while Claude Desktop switches.
pub(super) const EXIT_WAIT: &str = "Claude Desktop is switching accounts. Smartlog Claude Auto Switch will close when it is done.";

// A row's figures: "26% · 74%" (5-hour, weekly); "—" without any.
pub(super) fn figures(u: Option<&Use>) -> String {
    u.map_or("—".into(), |u| format!("{}% · {}%", u.five.u, u.week.u))
}

// When a used-up account can be used again; "not known" without a reset time.
pub(super) fn available_at(t: i64, now: i64) -> String {
    if t == i64::MAX { "not known".into() } else { clock(t, now) }
}

// A card's reset at `t`: "resets in 2h05m · 16:30".
pub(super) fn resets(t: i64, now: i64) -> String {
    format!("resets in {} · {}", left(t - now), clock(t, now))
}

// The question before Claude Desktop switches to `email`.
pub(super) fn desk_confirm(email: &str) -> String {
    format!("Claude Desktop will restart signed in as {email}. Any reply in progress is cut off. Continue?")
}
