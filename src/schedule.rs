// When each account's usage is asked for next (docs/specs/check-schedule.md): rarely when
// it is not in use or far from a limit, every minute near one, never while a limit is full.

use crate::switch::rank::{usage_now, K};
use crate::*;

const MIN: i64 = 60_000;
// In use: every SLOWEST at 0%, falling to every FASTEST at FAST_FROM (5-hour points) and over.
pub(crate) const SLOWEST_MS: i64 = 10 * MIN;
pub(crate) const FASTEST_MS: i64 = MIN;
const FAST_FROM: f64 = 90.0;
// After an error other than a 429 (those back off: backoff_ms).
pub(crate) const RETRY_MS: i64 = 2 * MIN;

// How near a limit, in 5-hour points: the 5-hour %, or the weekly one counted as 5-hour
// points (1% of a week is about K% of 5 hours).
pub(crate) fn nearness(u: &Usage) -> f64 {
    (u.five.u as f64).max(100.0 - (100 - u.week.u) as f64 * K)
}

// After figures `u` fetched at `t`. A window at 100% cannot change until it resets: no
// question before then (plus a minute, so Anthropic has reset it).
pub(crate) fn next_check_at(u: &Usage, in_use: bool, t: i64) -> i64 {
    let full = [&u.five, &u.week].into_iter().filter(|w| w.u >= 100).filter_map(|w| w.resets_at).max();
    if let Some(r) = full.filter(|r| *r > t) {
        return r + MIN;
    }
    if !in_use {
        return t + SLOWEST_MS;
    }
    let p = nearness(u).clamp(0.0, FAST_FROM);
    t + SLOWEST_MS - ((SLOWEST_MS - FASTEST_MS) as f64 * p / FAST_FROM).round() as i64
}

// Which accounts are in use: the Claude CLI login `active`, and the account Claude Desktop
// is signed in to (`desk`, its uuid). One that starts or stops being used gets its next
// check worked out again from its last figures, unless it has an error (its retry time
// stands); never before the end of a 429's wait.
pub(crate) fn mark_in_use(v: &mut [Account], active: Option<&str>, desk: Option<&str>, t: i64) {
    for a in v.iter_mut() {
        let on = active == Some(a.id.as_str()) || (desk.is_some() && a.account_uuid.as_deref() == desk);
        if on == a.in_use {
            continue;
        }
        if a.in_use && !on {
            a.left_at = t;
        }
        a.in_use = on;
        if a.error.is_none() {
            if let Some(u) = usage_now(a, t) {
                a.next_check = next_check_at(&u, on, u.fetched_at).max(a.slow_until);
            }
        }
    }
}
