// Checking GitHub for a newer release and installing it (docs/specs/update.md).
// github.rs: the release and its setup (pure parts); install.rs: download and run it.

use crate::*;

mod github;
mod install;
pub(crate) use github::*;

// One source for the version: Cargo.toml.
pub(crate) const VERSION: &str = env!("CARGO_PKG_VERSION");

const FIRST_LOOK: Duration = Duration::from_secs(60);
const LOOK_EVERY: Duration = Duration::from_secs(3600);
const CHECK_EVERY_MS: i64 = 3 * 24 * 3_600_000;

// Whether the background checker asks GitHub now: 3 days after its last answer.
pub(crate) fn check_due(last: Option<i64>, t: i64) -> bool {
    last.is_none_or(|l| t - l >= CHECK_EVERY_MS)
}

#[derive(Serialize, Clone, Copy, PartialEq, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) enum Phase {
    #[default]
    Idle,
    Checking,
    UpToDate,
    Available,
    Downloading,
    Installing,
    Error,
}

// What /api/state shows as "update".
#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Status {
    #[serde(rename = "status")]
    pub(crate) phase: Phase,
    pub(crate) latest: Option<String>,
    // The release page ("What's new").
    pub(crate) url: Option<String>,
    pub(crate) checked_at: Option<i64>,
    pub(crate) error: Option<String>,
    // The newest version already written to the events log, so a later check does not
    // repeat "Update X found".
    #[serde(skip)]
    pub(crate) logged: Option<String>,
}

// The background checker: looks a minute after start, then hourly, and checks once 3
// days have passed since GitHub last answered (saved, so restarts do not check again).
// Errors only go to the status and the log; the next look tries again.
pub(crate) fn start(app: Arc<App>) {
    if app.demo {
        return;
    }
    thread::spawn(move || {
        thread::sleep(FIRST_LOOK);
        loop {
            if check_due(app.st.lock().unwrap().update_checked_at, now()) {
                app.auto_update();
            }
            thread::sleep(LOOK_EVERY);
        }
    });
}

impl App {
    fn set_update(&self, f: impl FnOnce(&mut Status)) {
        f(&mut self.update.lock().unwrap());
    }

    // Claude Desktop being switched, or a browser sign-in waiting: the setup closes this
    // app, which would cut either short.
    pub(crate) fn update_blocked(&self) -> Option<&'static str> {
        if self.switching.load(Ordering::SeqCst) {
            return Some("A Claude Desktop switch is running; install the update when it is done.");
        }
        if self.login.lock().unwrap().as_ref().is_some_and(|p| now() - p.started < login::PENDING_MS) {
            return Some("A sign-in is under way; install the update when it is done.");
        }
        None
    }

    // Marks the status "checking"; false when there is nothing to start (demo, busy).
    pub(crate) fn begin_check(&self) -> bool {
        if self.demo {
            return false;
        }
        let mut u = self.update.lock().unwrap();
        if matches!(u.phase, Phase::Checking | Phase::Downloading | Phase::Installing) {
            return false;
        }
        u.phase = Phase::Checking;
        true
    }

    // Asks GitHub for the latest release; blocking (up to the agent's 20 s).
    pub(crate) fn check_update(&self) -> Result<(), String> {
        if !self.begin_check() {
            return Ok(());
        }
        self.finish_check()
    }

    pub(crate) fn finish_check(&self) -> Result<(), String> {
        let res = latest_release(&self.http);
        let t = now();
        let found = match &res {
            Ok(r) => {
                let tag = r["tag_name"].as_str().unwrap_or("");
                let latest = tag.trim_start_matches('v').to_string();
                let url = r["html_url"].as_str().filter(|u| u.starts_with("https://github.com/")).map(String::from);
                let newer = newer(tag, VERSION);
                let mut u = self.update.lock().unwrap();
                *u = Status {
                    phase: if newer { Phase::Available } else { Phase::UpToDate },
                    latest: Some(latest.clone()).filter(|l| !l.is_empty()),
                    url,
                    checked_at: Some(t),
                    error: None,
                    logged: u.logged.take(),
                };
                let first = newer && u.logged.as_deref() != Some(latest.as_str());
                if first {
                    u.logged = Some(latest.clone());
                }
                first.then_some(latest)
            }
            Err(e) => {
                self.set_update(|u| {
                    u.phase = Phase::Error;
                    u.checked_at = Some(t);
                    u.error = Some(e.clone());
                });
                None
            }
        };
        let mut st = self.st.lock().unwrap();
        if res.is_ok() {
            st.update_checked_at = Some(t);
        }
        if let Some(v) = found {
            self.log(&mut st, format!("Update {v} found"));
        }
        self.save(&st);
        drop(st);
        res.map(|_| ())
    }

    // One round of the background checker: check, then install at once when allowed.
    // A check or install already running: check_update leaves it alone.
    fn auto_update(&self) {
        if self.check_update().is_err() || self.update.lock().unwrap().phase != Phase::Available {
            return;
        }
        let auto = self.st.lock().unwrap().settings.auto_update;
        // Blocked (a switch or a sign-in): the next check tries again.
        if auto && self.update_blocked().is_none() {
            let _ = self.install_update();
        }
    }
}
