// When a parked account's sign-in may be refreshed (docs/specs/idle-refresh.md): only when
// an account in use is near a limit, or the user asks, and not right after it was in use.

use crate::*;

pub(crate) const NEED_FROM: f64 = 80.0;
pub(crate) const LEFT_GUARD_MS: i64 = 15 * 60_000;

// An account in use is near a limit (or at one): the switch will need fresh figures for
// the others.
pub(crate) fn needed(accounts: &[Account], t: i64) -> bool {
    accounts.iter().filter(|a| a.in_use).filter_map(|a| crate::switch::rank::usage_now(a, t)).any(|u| {
        u.five.u >= 100 || u.week.u >= 100 || super::schedule::nearness(&u) >= NEED_FROM
    })
}

// A parked account's sign-in may be refreshed: needed or asked for, and not while an old
// window may still hold it (it left use less than LEFT_GUARD_MS ago; 0: never this run).
pub(crate) fn may_refresh(need: bool, force: bool, left_at: i64, t: i64) -> bool {
    (need || force) && (left_at == 0 || t - left_at >= LEFT_GUARD_MS)
}
