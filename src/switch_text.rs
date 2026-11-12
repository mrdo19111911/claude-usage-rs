// The events-list wording of a Claude Code switch, for office staff
// (docs/REVIEW_DE_HIEU.md section 5, "Names to use everywhere").
//
// Spec:
// - switched(from, to, why): "Claude Code: switched from a@x to b@x (why)"; with no
//   account in use before, "Claude Code: switched to b@x (why)".
// - BY_YOU: the reason of a switch asked for in the dashboard or the widget.
// - limit_reason(u, s): why auto-switch leaves the account in use, as
//   "5-hour limit at 95%" / "weekly limit at 99%", or "... used up" at 100%;
//   None while both limits are under their thresholds (same test as switch_reason).
// - sooner(email, h): the proactive switch to the account whose weekly limit resets
//   first: "b@x's weekly limit resets sooner, in 30 h, so it is used first".
// - no_room(who, which): auto-switch must leave but cannot; `which` is switch_reason's
//   "5-hour 95%": "a@x is at 95% of its 5-hour limit, and no other account has any left".
// - desktop_switching(name, u): Claude Desktop's auto-switch line.
// - chats_copied / CHATS_NOT_COPIED: Claude Desktop's Claude Code chat list on a switch
//   (the raw error goes to stderr at the call site).
// - check_failed(e): the check with Anthropic before a switch failed, without the raw text;
//   SLOWED for a 429, and auto_failed(e) turns it into "Auto-switch waited: ...".
// - Plain refusals: OPENED_BY_CLAUDE, STILL_READING, OTHER_ACCOUNT, NOT_FOUND, ...
// Nothing parses these strings except auto_failed (SLOWED, by equality); they are only
// shown (settings.js #events, alerts). tidy.js reads accounts' `error`, never these.

use crate::*;

pub(crate) const BY_YOU: &str = "by you";

pub(crate) fn switched(from: Option<&str>, to: &str, why: &str) -> String {
    match from {
        Some(f) => format!("Claude Code: switched from {f} to {to} ({why})"),
        None => format!("Claude Code: switched to {to} ({why})"),
    }
}

fn at(name: &str, u: i64) -> String {
    if u >= 100 {
        format!("{name} limit used up")
    } else {
        format!("{name} limit at {u}%")
    }
}

pub(crate) fn limit_reason(u: &Usage, s: &Settings) -> Option<String> {
    if u.five.u >= s.threshold {
        Some(at("5-hour", u.five.u))
    } else if u.week.u >= s.week_threshold {
        Some(at("weekly", u.week.u))
    } else {
        None
    }
}

pub(crate) fn sooner(email: &str, h: i64) -> String {
    format!("{email}'s weekly limit resets sooner, in {h} h, so it is used first")
}

pub(crate) const DESKTOP_ACCOUNT: &str = "Claude Desktop's account";

pub(crate) fn no_room(who: &str, which: &str) -> String {
    let what = match which.split_once(' ') {
        Some((name, "100%")) => format!("has used up its {name} limit"),
        Some((name, pct)) => format!("is at {pct} of its {name} limit"),
        None => format!("is at {which}"),
    };
    format!("{who} {what}, and no other account has any left")
}

pub(crate) fn desktop_switching(name: &str, u: i64) -> String {
    format!("Claude Desktop: switching automatically ({})", at(name, u))
}

pub(crate) fn chats_copied(added: usize, updated: usize, removed: usize) -> String {
    let gone = if removed > 0 { format!(", {removed} removed") } else { String::new() };
    format!("Claude Desktop: Claude Code chat list copied ({added} new, {updated} updated{gone})")
}

pub(crate) const CHATS_NOT_COPIED: &str = "Claude Desktop: the Claude Code chat list could not be copied. It tries again at the next switch.";
pub(crate) const DESKTOP_SIGNED_OUT: &str = "Claude Desktop is signed out: sign in there with the account to add";
pub(crate) const OPENED_BY_CLAUDE: &str = "This app was opened by Claude, so restarting Claude Desktop would close it too. Open it from the Start menu instead.";
pub(crate) const STILL_READING: &str = "Still reading which account Claude Code uses; try again in a few seconds";
pub(crate) const NOT_FOUND: &str = "This account is no longer in the list. Reload the page.";
pub(crate) const SOMETHING_WRONG: &str = "Something went wrong; reload the page.";
pub(crate) const NAME_NEEDED: &str = "Type a name for this account.";
pub(crate) const OTHER_ACCOUNT: &str = "This account's saved sign-in now belongs to a different account. Sign in to it again.";
pub(crate) const SLOWED: &str = "Anthropic asked the app to check less often, so it was not switched. Try again in a few minutes.";

pub(crate) fn check_failed(e: &ApiErr) -> String {
    match e {
        ApiErr::Slow(_) => SLOWED.into(),
        ApiErr::Auth(_) => "Anthropic did not accept this account's sign-in, so it was not switched. Sign in to it again.".into(),
        ApiErr::Other(_) => "Could not reach Anthropic to check this account, so it was not switched. Try again shortly.".into(),
    }
}

pub(crate) fn auto_failed(e: &str) -> String {
    if e == SLOWED {
        "Auto-switch waited: Anthropic asked the app to check less often".into()
    } else {
        format!("Auto-switch failed: {e}")
    }
}
