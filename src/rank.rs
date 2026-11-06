// Which account auto-switch uses, and the order accounts are shown in: the weekly quota
// that expires first is used first (docs/specs/auto-switch-deadline.md).

use crate::switch::{has_room, switch_reason};
use crate::*;
use std::cmp::Ordering as Ord;

// 1% of the weekly window is ~K x 1% of the 5-hour one (measured 6.72).
pub(crate) const K: f64 = 6.7;
// A proactive switch needs a deadline this much sooner than the account in use's.
pub(crate) const MARGIN_MS: i64 = HOUR;
// "Useful": the 5-hour window at least this many points under the threshold.
pub(crate) const MIN_FIVE_ROOM: i64 = 10;

pub(crate) enum Pick {
    Stay,
    Switch(String, String),
    // The account in use reached a limit (which) and no other one has room.
    NoRoom(String),
}

// A window's % now: one whose reset has passed counts as 0.
fn eff(w: &Win, now: i64) -> Win {
    let gone = w.resets_at.is_some_and(|r| r <= now);
    Win {
        u: if gone { 0 } else { w.u },
        resets_at: if gone { None } else { w.resets_at },
    }
}

pub(crate) fn usage_now(a: &Account, now: i64) -> Option<Usage> {
    a.usage.as_ref().map(|u| Usage {
        five: eff(&u.five, now),
        week: eff(&u.week, now),
        fetched_at: u.fetched_at,
        scoped: vec![],
    })
}

fn usable(a: &Account) -> bool {
    a.enabled && a.error.is_none()
}

// Eligible: usable, checked, under both thresholds.
fn eligible(a: &Account, s: &Settings, now: i64) -> Option<Usage> {
    usage_now(a, now).filter(|u| usable(a) && has_room(u, s))
}

pub(crate) fn useful(u: &Usage, s: &Settings) -> bool {
    u.five.u <= s.threshold - MIN_FIVE_ROOM
}

// Weekly % that can be used now: the week left, capped by the 5-hour window left.
fn room_now(u: &Usage) -> f64 {
    ((100 - u.week.u) as f64).min((100 - u.five.u) as f64 / K)
}

// Earliest deadline (unknown last), then more room now, then the id. To the minute: the
// same reset comes back ~1.5 s apart (better-ccflare, docs/SO_SANH_AUTO_SWITCH.md).
fn rank(a: (&Account, &Usage), b: (&Account, &Usage)) -> Ord {
    let d = |u: &Usage| u.week.resets_at.map_or(i64::MAX, |r| r / 60_000);
    d(a.1)
        .cmp(&d(b.1))
        .then(room_now(b.1).total_cmp(&room_now(a.1)))
        .then(a.0.id.cmp(&b.0.id))
}

// The eligible accounts, ranked.
pub(crate) fn ranked<'a>(v: &'a [Account], s: &Settings, now: i64) -> Vec<(usize, Usage)> {
    let mut r: Vec<(usize, Usage)> = v
        .iter()
        .enumerate()
        .filter_map(|(i, a)| eligible(a, s, now).map(|u| (i, u)))
        .collect();
    r.sort_by(|x, y| rank((&v[x.0], &x.1), (&v[y.0], &y.1)));
    r
}

// The account to switch to from `active`, with the reason.
pub(crate) fn pick(v: &[Account], active: &str, s: &Settings, now: i64) -> Pick {
    let Some(cur) = v.iter().find(|a| a.id == active) else {
        return Pick::Stay;
    };
    let Some(cur_u) = usage_now(cur, now) else {
        return Pick::Stay;
    };
    if !s.auto_switch {
        return Pick::Stay;
    }
    let others: Vec<(usize, Usage)> = ranked(v, s, now)
        .into_iter()
        .filter(|(i, _)| v[*i].id != active)
        .collect();
    if let Some(which) = switch_reason(&cur_u, s) {
        let best = if s.use_soonest_reset {
            others
                .iter()
                .find(|(_, u)| useful(u, s))
                .or(others.first())
                .map(|(i, _)| *i)
        } else {
            others
                .iter()
                .max_by(|x, y| {
                    old_room(&x.1)
                        .cmp(&old_room(&y.1))
                        .then(v[y.0].id.cmp(&v[x.0].id))
                })
                .map(|(i, _)| *i)
        };
        return match best {
            Some(i) => Pick::Switch(v[i].id.clone(), format!("auto: {which}")),
            None => Pick::NoRoom(which),
        };
    }
    if !s.use_soonest_reset || !usable(cur) {
        return Pick::Stay;
    }
    let cur_deadline = cur_u.week.resets_at.unwrap_or(i64::MAX);
    match others.iter().find(|(_, u)| useful(u, s)) {
        Some((i, u))
            if u.week
                .resets_at
                .is_some_and(|d| d.saturating_add(MARGIN_MS) <= cur_deadline) =>
        {
            let h = (u.week.resets_at.unwrap() - now + HOUR / 2) / HOUR;
            Pick::Switch(
                v[*i].id.clone(),
                format!("auto: weekly resets in {h}h, sooner"),
            )
        }
        _ => Pick::Stay,
    }
}

// The old rule's room: the smaller of the two windows' room.
fn old_room(u: &Usage) -> i64 {
    (100 - u.five.u).min(100 - u.week.u)
}

// The order accounts are shown in (indexes into `v`): the ranking (or, setting off, the
// saved order), with the accounts at 100% of a limit last, soonest free again first.
pub(crate) fn order(v: &[Account], s: &Settings, now: i64) -> Vec<usize> {
    let shown = if s.use_soonest_reset { ranked_order(v, s, now) } else { (0..v.len()).collect() };
    let full = |i: &usize| usage_now(&v[*i], now).and_then(|u| full_until(&u));
    let (mut out, mut last): (Vec<usize>, Vec<usize>) = shown.into_iter().partition(|i| full(i).is_none());
    last.sort_by_key(|i| full(i));
    out.extend(last);
    out
}

// When an account at 100% of a limit is free again: the latest reset among its windows at
// 100% (i64::MAX when unknown); None when no window is at 100%.
fn full_until(u: &Usage) -> Option<i64> {
    [&u.five, &u.week].into_iter().filter(|w| w.u >= 100).map(|w| w.resets_at.unwrap_or(i64::MAX)).max()
}

// The ranking, then the accounts at a threshold (soonest usable again first), then
// disabled, errored and unchecked ones.
fn ranked_order(v: &[Account], s: &Settings, now: i64) -> Vec<usize> {
    let mut out: Vec<usize> = ranked(v, s, now).into_iter().map(|(i, _)| i).collect();
    let mut held: Vec<(i64, usize)> = vec![];
    let mut rest = vec![];
    for (i, a) in v.iter().enumerate().filter(|(i, _)| !out.contains(i)) {
        match usage_now(a, now).filter(|_| usable(a)) {
            Some(u) => held.push((free_at(&u, s), i)),
            None => rest.push(i),
        }
    }
    held.sort();
    out.extend(held.into_iter().map(|(_, i)| i));
    out.extend(rest);
    out
}

// When an account at a threshold is usable again: the latest reset among its windows at
// one (i64::MAX when unknown).
fn free_at(u: &Usage, s: &Settings) -> i64 {
    let at = |w: &Win, limit: i64| (w.u >= limit).then(|| w.resets_at.unwrap_or(i64::MAX));
    [at(&u.five, s.threshold), at(&u.week, s.week_threshold)]
        .into_iter()
        .flatten()
        .max()
        .unwrap_or(i64::MAX)
}

#[cfg(test)]
#[path = "rank_tests.rs"]
mod tests;
