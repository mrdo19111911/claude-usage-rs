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

#[cfg(test)]
mod tests {
    use super::*;

    const ME: &str = "Smartlog Claude Auto Switch";

    // Asked 2026-11-12: "FULL FREE Again là gì ... sao không viết là đã dùng hết. available at".
    #[test]
    fn the_used_up_section_says_so_in_plain_words() {
        assert_eq!(USED_UP, "USED UP · AVAILABLE AT");
        assert_eq!(available_at(i64::MAX, 0), "not known");
    }

    // The review: both figures with a %, and a key saying which is which.
    #[test]
    fn rows_mark_both_figures() {
        assert_eq!(FIGS_KEY, "5-hour · weekly");
        let u = Use { five: Win { u: 26, reset: None }, week: Win { u: 74, reset: None } };
        assert_eq!(figures(Some(&u)), "26% · 74%");
        assert_eq!(figures(None), "—");
    }

    #[test]
    fn section_titles_say_what_the_accounts_are() {
        assert_eq!(IN_USE, "IN USE NOW");
        assert_eq!(SWITCH_TO, "SWITCH TO");
    }

    #[test]
    fn states_without_figures_say_usage() {
        assert_eq!(NOT_CHECKED, "Usage not checked yet");
        assert_eq!(NO_USE, "no use yet");
    }

    #[test]
    fn a_reset_says_it_resets() {
        let now = 1_760_000_000_000;
        let t = now + 2 * 3_600_000 + 5 * 60_000;
        assert_eq!(resets(t, now), format!("resets in 2h05m · {}", clock(t, now)));
    }

    #[test]
    fn the_desktop_question_says_restart() {
        let want = "Claude Desktop will restart signed in as b@x.com. Any reply in progress is cut off. Continue?";
        assert_eq!(desk_confirm("b@x.com"), want);
    }

    #[test]
    fn the_program_has_one_name() {
        assert_eq!(APP, ME);
        assert_eq!(EXIT_WAIT, format!("Claude Desktop is switching accounts. {ME} will close when it is done."));
    }

    #[test]
    fn menu_items_say_what_they_do() {
        assert_eq!(PIN_ITEM, "Keep widget on screen");
        assert_eq!(EXIT_ITEM, "Exit (stops auto-switch)");
    }
}
