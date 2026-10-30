// Checking each account's usage on its own schedule.

use crate::*;

impl App {
    pub(crate) fn poll_account(&self, id: &str, force: bool) {
        let (creds, is_active, known, slow_until, base) = {
            let st = self.st.lock().unwrap();
            match st.accounts.iter().find(|a| a.id == id) {
                Some(a) if force || now() >= a.next_check => {
                    (a.creds.clone(), st.active_id.as_deref() == Some(id), a.account_uuid.is_some(), a.slow_until, st.settings.poll_seconds as i64 * 1000)
                }
                _ => return,
            }
        };
        // Anthropic asked this login to slow down: its last figures stand until then.
        if now() < slow_until {
            return;
        }
        // Also the live login when it is not identified yet (active_id is then None).
        let rt = creds.get("refreshToken").filter(|v| v.is_string());
        let is_live = is_active || (rt.is_some() && self.read_live().is_some_and(|l| l.get("refreshToken") == rt));
        let res = self.fetch_usage(&creds, !is_live);
        // Accounts saved by older versions were named after ~/.claude.json, which can
        // be another account's: ask once whose the token is. Not tied to the usage
        // answer: the usage endpoint can be rate-limited (429) while this one answers.
        let who = if known {
            None
        } else {
            let fresh = res.as_ref().ok().and_then(|(_, c)| c.as_ref());
            self.get_profile(fresh.unwrap_or(&creds)).ok()
        };
        let mut st = self.st.lock().unwrap();
        let active_now = st.active_id.as_deref() == Some(id);
        let Some(a) = st.accounts.iter_mut().find(|a| a.id == id) else { return };
        let mut logged = None;
        if let Some((uuid, email)) = who {
            if a.email != email {
                println!("{} is {email}'s login; renamed", a.email);
            }
            a.account_uuid = Some(uuid);
            a.email = email;
            if a.oauth_account.get("accountUuid").and_then(Value::as_str) != a.account_uuid.as_deref() {
                a.oauth_account = Value::Null;
            }
        }
        match res {
            Ok((j, new_creds)) => {
                if let Some(c) = new_creds {
                    // Only a parked account is refreshed here (see fetch_usage).
                    a.creds = c;
                }
                let five = to_win(&j["five_hour"]);
                let week = to_win(&j["seven_day"]);
                let t = now();
                a.history.retain(|p| p.0 > t - 8 * 24 * HOUR);
                a.history.push((t, five.u, week.u));
                let u = Usage { five, week, fetched_at: t, scoped: to_scoped(&j) };
                a.next_check = next_check_at(&u, base, t);
                a.usage = Some(u);
                logged = usage_log::figures(a, active_now);
                a.error = None;
                a.raw = j;
                a.slow_strikes = 0;
            }
            Err(ApiErr::Slow(after)) => {
                // Asking again at once only prolongs it (seen: 429 with Retry-After 0).
                a.slow_strikes += 1;
                let wait = backoff_ms(after, a.slow_strikes);
                a.slow_until = now() + wait;
                a.next_check = a.slow_until;
                a.error = Some(format!("Anthropic asked to slow down (HTTP 429); next check in {} min", (wait + 59_999) / 60_000));
            }
            Err(e) => {
                a.error = Some(e.text());
                a.next_check = now() + base;
            }
        }
        // The usage log, after the state's lock is let go (usage_log.rs).
        drop(st);
        if let Some(fig) = logged {
            self.log_usage(fig);
        }
    }

    // `force`: every account now (Refresh now); else only those whose next check is due.
    pub(crate) fn poll_all(&self, force: bool) {
        if self.demo || self.polling.swap(true, Ordering::SeqCst) {
            return;
        }
        self.sync_active();
        let ids: Vec<String> = {
            let mut st = self.st.lock().unwrap();
            self.sync_desktop(&mut st);
            let t = now();
            st.accounts.iter().filter(|a| force || t >= a.next_check).map(|a| a.id.clone()).collect()
        };
        if ids.is_empty() {
            self.polling.store(false, Ordering::SeqCst);
            return;
        }
        for id in ids {
            self.poll_account(&id, force);
        }
        let next = self.pick_auto(&mut self.st.lock().unwrap());
        // Outside the lock: activate checks the login with Anthropic first.
        if let Some((id, reason)) = next {
            if let Err(e) = self.activate(&id, &reason) {
                let mut st = self.st.lock().unwrap();
                self.log(&mut st, format!("Auto-switch failed: {e}"));
            }
        }
        self.save(&self.st.lock().unwrap());
        self.last_poll.store(now(), Ordering::Relaxed);
        self.polling.store(false, Ordering::SeqCst);
    }
}

pub(crate) const SLOW_DOWN_MS: i64 = 5 * 60_000; // after a 429 that names no wait, as Claude Code waits
pub(crate) const MAX_WAIT_MS: i64 = 60 * 60_000;

// How long to leave a login alone after its `strikes`-th 429 in a row: the wait
// Anthropic named, else 5 min, doubling with each 429 in a row (to 40), at most an hour.
// A Retry-After of 0 names no wait: the next poll would only be refused again.
pub(crate) fn backoff_ms(after: Option<i64>, strikes: u32) -> i64 {
    let ms = match after {
        Some(ms) if ms > 0 => ms,
        _ => SLOW_DOWN_MS << strikes.saturating_sub(1).min(3),
    };
    ms.min(MAX_WAIT_MS)
}

// The last 7 days of figures for the charts: every point of the last 6 hours, older
// ones one per 10 minutes (the last of each), so the page stays small.
pub(crate) fn thin_history(h: &[(i64, i64, i64)], t: i64) -> Vec<(i64, i64, i64)> {
    let mut out: Vec<(i64, i64, i64)> = vec![];
    for &p in h.iter().filter(|p| p.0 > t - 7 * 24 * HOUR) {
        let old = p.0 <= t - 6 * HOUR;
        match out.last_mut() {
            Some(last) if old && last.0 / 600_000 == p.0 / 600_000 => *last = p,
            _ => out.push(p),
        }
    }
    out
}

// When to ask for an account's usage next, after figures fetched at `t`.
// A window at 100% cannot change until it resets: no question before then (plus a
// minute, so Anthropic has reset it). Otherwise every `base`, more often near a limit:
// from 70% half of it, from 85% a quarter, from 95% an eighth; never under 15 s.
pub(crate) fn next_check_at(u: &Usage, base_ms: i64, t: i64) -> i64 {
    let full: Vec<i64> = [&u.five, &u.week].iter().filter(|w| w.u >= 100).filter_map(|w| w.resets_at).collect();
    if let Some(r) = full.into_iter().max() {
        if r > t {
            return r + 60_000;
        }
    }
    let used = u.five.u.max(u.week.u);
    let every = match used {
        ..=69 => base_ms,
        70..=84 => base_ms / 2,
        85..=94 => base_ms / 4,
        _ => base_ms / 8,
    };
    t + every.max(15_000)
}
