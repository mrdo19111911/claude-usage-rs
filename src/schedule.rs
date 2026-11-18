// When each account's usage is asked for next (docs/specs/check-schedule.md): by the
// settings in poll_config.rs; never while a limit is full.

use crate::switch::rank::{usage_now, K};
use crate::*;

const MIN: i64 = 60_000;
// How long a parked account waiting for a sign-in refresh is left alone (idle-refresh.md).
pub(crate) const WAIT_REFRESH_MS: i64 = 10 * MIN;
// "Not checked unless asked": a check this far off (ms) is never reached in practice.
pub(crate) const NEVER_MS: i64 = 30 * 24 * HOUR;
// After an error other than a 429 (those back off: backoff_ms).
pub(crate) const RETRY_MS: i64 = 2 * MIN;

// How near a limit, in 5-hour points: the 5-hour %, or the weekly one counted as 5-hour
// points (1% of a week is about K% of 5 hours). Used by the switch and idle-refresh rules.
pub(crate) fn nearness(u: &Usage) -> f64 {
    (u.five.u as f64).max(100.0 - (100 - u.week.u) as f64 * K)
}

// After figures `u` fetched at `t`. A window at 100% cannot change until it resets: no
// question before then (plus a minute, so Anthropic has reset it). Else in use: by the
// 5-hour % (the weekly one only from its high mark; poll_config.rs); not in use: a random wait, or never when
// other accounts are not asked about. `rng` draws the random wait.
pub(crate) fn next_check_at(u: &Usage, in_use: bool, t: i64, cfg: &PollCfg, rng: &mut dyn FnMut() -> u64) -> i64 {
    let full = [&u.five, &u.week].into_iter().filter(|w| w.u >= 100).filter_map(|w| w.resets_at).max();
    if let Some(r) = full.filter(|r| *r > t) {
        return r + MIN;
    }
    if in_use {
        return t + interval_for(u.five.u, u.week.u, cfg);
    }
    if cfg.poll_others {
        t + idle_interval(cfg, rng)
    } else {
        t + NEVER_MS
    }
}

// Whether the account is asked now: Check now (`force`) asks all; else its time has come
// and it is in use, or other accounts are asked about, or it has never had figures.
pub(crate) fn is_due(a: &Account, t: i64, force: bool, cfg: &PollCfg) -> bool {
    let first = a.usage.is_none() && a.error.is_none();
    force || (t >= a.next_check && (a.in_use || cfg.poll_others || first))
}

// A random generator for one account's next check: the clock and the account's id.
pub(crate) fn rng_for(id: &str) -> XorShift {
    XorShift::from_clock(id.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3)))
}

// Which accounts are in use: the Claude CLI login `active`, and the account Claude Desktop
// is signed in to (`desk`, its uuid). One that starts or stops being used gets its next
// check worked out again from its last figures, unless it has an error (its retry time
// stands); never before the end of a 429's wait.
pub(crate) fn mark_in_use(v: &mut [Account], active: Option<&str>, desk: Option<&str>, t: i64, cfg: &PollCfg, rng: &mut dyn FnMut() -> u64) {
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
                a.next_check = next_check_at(&u, on, u.fetched_at, cfg, rng).max(a.slow_until);
            }
        }
    }
}
