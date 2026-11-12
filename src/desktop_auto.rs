// Claude Desktop's auto-switch: when the account it is signed in to reaches DESKTOP_AT
// of a limit, move it to the first ranked account it may use (docs/specs/account-cards.md).
// The switch closes and reopens Desktop, so it waits longer than Claude CLI's threshold.

use crate::switch::rank::{ranked, useful, usage_now, Pick};
use crate::*;

// The least time between two Desktop auto-switches (each closes and restarts Desktop).
pub(crate) const GAP_MS: i64 = 10 * 60_000;
// From this % of the 5-hour or weekly limit (asked 2026-11-07; it was 100).
pub(crate) const DESKTOP_AT: i64 = 98;

// The account Claude Desktop moves to from `signed_in`, with the log line. `has_sign_in`:
// whether a Desktop sign-in is saved for a uuid; `last`: the last Desktop auto-switch (ms).
pub(crate) fn pick_desktop(v: &[Account], desk: &[DesktopAccount], signed_in: Option<&str>, has_sign_in: &dyn Fn(&str) -> bool, s: &Settings, now: i64, last: i64) -> Pick {
    let Some(cur) = signed_in else { return Pick::Stay };
    if !s.auto_switch || now - last < GAP_MS {
        return Pick::Stay;
    }
    // Its figures: the Claude CLI login of the same account.
    let Some(u) = v.iter().find(|a| a.account_uuid.as_deref() == Some(cur)).and_then(|a| usage_now(a, now)) else { return Pick::Stay };
    let which = if u.five.u >= DESKTOP_AT {
        format!("5-hour {}%", u.five.u)
    } else if u.week.u >= DESKTOP_AT {
        format!("weekly {}%", u.week.u)
    } else {
        return Pick::Stay;
    };
    let may = |uuid: &str| uuid != cur && desk.iter().any(|d| d.uuid == uuid && d.auto) && has_sign_in(uuid);
    let best = ranked(v, s, now).into_iter().filter(|(_, u)| useful(u, s)).find_map(|(i, _)| v[i].account_uuid.clone().filter(|x| may(x)));
    match best {
        Some(uuid) => Pick::Switch(uuid, format!("Claude Desktop: auto-switch ({which})")),
        None => Pick::NoRoom(which),
    }
}

impl App {
    // Each poll, after Claude CLI's auto-switch. Takes the state lock itself.
    pub(crate) fn desktop_auto(self: &Arc<Self>) {
        if self.demo || self.switching.load(Ordering::SeqCst) {
            return;
        }
        let Some(dir) = &self.desktop_dir else { return };
        let signed_in = desktop::signed_in(dir);
        let mut st = self.st.lock().unwrap();
        // The 4:00 renewal moves Desktop itself until it is back (docs/specs/desktop-renew.md).
        if st.desktop_renew.renewing.is_some() {
            return;
        }
        // A saved sign-in whose session has ended is skipped (docs/specs/desktop-signed-out.md).
        let has = |u: &str| self.desktop_store.usable(u, now());
        match pick_desktop(&st.accounts, &st.desktop, signed_in.as_deref(), &has, &st.settings, now(), self.desktop_auto_at.load(Ordering::SeqCst)) {
            Pick::Stay => {}
            Pick::NoRoom(which) => {
                let msg = format!("Claude Desktop hit {which} but no other account has room");
                if st.events.first().map(|e| &e.msg) != Some(&msg) {
                    self.log(&mut st, msg);
                }
            }
            Pick::Switch(uuid, why) => {
                self.log(&mut st, why);
                self.desktop_auto_at.store(now(), Ordering::SeqCst);
                drop(st);
                // It spawns its own thread, which takes the lock.
                if let Err(e) = self.start_desktop_switch(Some(uuid)) {
                    let mut st = self.st.lock().unwrap();
                    self.log(&mut st, format!("Claude Desktop auto-switch failed: {e}"));
                }
            }
        }
    }

    // POST /api/desktop/<uuid>/auto {enabled}: whether auto-switch may move Desktop there.
    pub(crate) fn set_desktop_auto(&self, uuid: &str, on: bool) -> Result<(), String> {
        let mut st = self.st.lock().unwrap();
        st.desktop.iter_mut().find(|d| d.uuid == uuid).ok_or("not found")?.auto = on;
        self.save(&st);
        Ok(())
    }
}

#[cfg(test)]
#[path = "desktop_auto_tests.rs"]
mod tests;
