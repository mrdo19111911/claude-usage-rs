// Renewing saved Claude Desktop sign-ins at 4:00 (docs/specs/desktop-renew.md, prior art
// docs/SO_SANH_DESKTOP_RENEW.md). claude.ai keeps a sign-in 28 days from its last use; a
// saved one due within 7 days is opened in Desktop for a few minutes, then Desktop goes
// back to the account it was on.

use crate::*;
use serde::{Deserialize, Serialize};

#[path = "local_clock.rs"]
mod local_clock;
pub(crate) use local_clock::Clock;

const DAY_MS: i64 = 86_400_000;
pub(crate) const RENEW_HOUR: u32 = 4;
pub(crate) const RENEW_WITHIN: i64 = 7 * DAY_MS;
// Due this soon, it is renewed even while a Code session runs.
pub(crate) const MUST_WITHIN: i64 = 2 * DAY_MS;
// How long Desktop stays on the account being renewed, from the switch's start.
pub(crate) const STAY_MS: i64 = 3 * 60_000;
// Renewed: it now ends at least this far away.
const RENEWED_MS: i64 = 20 * DAY_MS;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct Renewing {
    pub(crate) target: String,
    pub(crate) back: String,
    pub(crate) at: i64,
}

/// Kept in state.json (`desktopRenew`): the renewal under way, and who was tried on `day`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub(crate) struct RenewState {
    pub(crate) renewing: Option<Renewing>,
    pub(crate) day: i64,
    pub(crate) tried: Vec<String>,
}

#[derive(Debug, PartialEq)]
pub(crate) enum Step {
    Wait,
    Renew(String),
    Back(String),
    Done,
}

/// What to do now. `until`: when a saved sign-in ends (None: ended or unknown).
pub(crate) fn next_step(on: bool, st: &RenewState, signed_in: Option<&str>, desk: &[DesktopAccount], until: &dyn Fn(&str) -> Option<i64>, code_running: bool, c: &Clock) -> Step {
    if let Some(r) = &st.renewing {
        return if c.ms - r.at < STAY_MS {
            Step::Wait
        } else if signed_in == Some(r.back.as_str()) {
            Step::Done
        } else {
            Step::Back(r.back.clone())
        };
    }
    let Some(cur) = signed_in.filter(|_| on && c.hour == RENEW_HOUR) else { return Step::Wait };
    let tried = |u: &str| st.day == c.day && st.tried.iter().any(|t| t == u);
    let due = desk
        .iter()
        .filter(|d| d.uuid != cur && !tried(&d.uuid))
        .filter_map(|d| until(&d.uuid).filter(|u| *u > c.ms && *u - c.ms < RENEW_WITHIN).map(|u| (u, d.uuid.clone())))
        .min();
    match due {
        Some((u, _)) if code_running && u - c.ms >= MUST_WITHIN => Step::Wait,
        Some((_, uuid)) => Step::Renew(uuid),
        None => Step::Wait,
    }
}

impl App {
    // Each poll, after Desktop's auto-switch. Takes the state lock itself.
    pub(crate) fn desktop_renew(self: &Arc<Self>) {
        if self.demo || self.switching.load(Ordering::SeqCst) {
            return;
        }
        let Some(dir) = self.desktop_dir.clone() else { return };
        let signed_in = desktop::signed_in(&dir);
        let c = Clock::at(now());
        let code = desktop_code_running(&dir);
        let until = |u: &str| match self.desktop_store.saved_session(u, c.ms) {
            Some(desktop::Session::Live(x)) => x,
            _ => None,
        };
        let mut st = self.st.lock().unwrap();
        let days = |u: Option<i64>| u.map(|u| ((u - c.ms) / DAY_MS).to_string()).unwrap_or_else(|| "?".into());
        match next_step(st.settings.renew_desktop, &st.desktop_renew, signed_in.as_deref(), &st.desktop, &until, code, &c) {
            Step::Wait => {}
            Step::Renew(target) => {
                let r = &mut st.desktop_renew;
                if r.day != c.day {
                    (r.day, r.tried) = (c.day, vec![]);
                }
                r.tried.push(target.clone());
                r.renewing = Some(Renewing { target: target.clone(), back: signed_in.clone().unwrap_or_default(), at: c.ms });
                let msg = format!("Claude Desktop: renewing {}'s saved sign-in ({} days left)", label(&st, &target), days(until(&target)));
                self.log(&mut st, msg);
                self.save(&st);
                drop(st);
                if let Err(e) = self.start_desktop_switch(Some(target)) {
                    let mut st = self.st.lock().unwrap();
                    st.desktop_renew.renewing = None;
                    self.log(&mut st, format!("Claude Desktop: renewal not started: {e}"));
                    self.save(&st);
                }
            }
            Step::Back(back) => {
                self.desktop_auto_at.store(c.ms, Ordering::SeqCst);
                drop(st);
                if let Err(e) = self.start_desktop_switch(Some(back)) {
                    let mut st = self.st.lock().unwrap();
                    let msg = format!("Claude Desktop: going back after a renewal failed: {e}");
                    if st.events.first().map(|e| &e.msg) != Some(&msg) {
                        self.log(&mut st, msg);
                    }
                }
            }
            Step::Done => {
                let Some(r) = st.desktop_renew.renewing.take() else { return };
                let u = until(&r.target);
                let ok = u.is_some_and(|u| u - c.ms >= RENEWED_MS);
                let how = if ok { "renewed" } else { "was not renewed (tried again tomorrow at 4:00)" };
                let msg = format!("Claude Desktop: {}'s sign-in {how}: {} days left", label(&st, &r.target), days(u));
                self.log(&mut st, msg);
                self.save(&st);
            }
        }
    }
}

fn label(st: &State, uuid: &str) -> String {
    st.desktop.iter().find(|d| d.uuid == uuid).map(|d| d.label.clone()).unwrap_or_else(|| uuid.chars().take(8).collect())
}

#[cfg(windows)]
fn desktop_code_running(dir: &Path) -> bool {
    desktop::RealApp { data_dir: dir.to_path_buf() }.code_running()
}
#[cfg(not(windows))]
fn desktop_code_running(_dir: &Path) -> bool {
    false
}

#[cfg(test)]
#[path = "desktop_renew_tests.rs"]
mod tests;
