// Checking GitHub for a newer release and installing it (docs/specs/update.md).
// check.rs: asking GitHub; github.rs: the release and its setup (pure parts);
// install.rs: download and run it.

use crate::*;

mod check;
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
// days have passed since GitHub last answered with nothing to install (saved, so
// restarts do not check again). Errors only go to the status and the log; the next look
// tries again, as it does a found version not installed yet (check.rs).
pub(crate) fn start(app: Arc<App>) {
    if app.demo {
        return;
    }
    app.update.lock().unwrap().checked_at = app.st.lock().unwrap().update_checked_at;
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

    // One round of the background checker: check, then install at once when allowed.
    // A check or install already running: check_update leaves it alone.
    fn auto_update(&self) {
        if self.check_update().is_err() || self.update.lock().unwrap().phase != Phase::Available {
            return;
        }
        let auto = self.st.lock().unwrap().settings.auto_update;
        // Blocked (a switch or a sign-in), or the install failed: the check time was not
        // saved (check.rs), so the next hourly look tries again.
        if auto && self.update_blocked().is_none() {
            let _ = self.install_update();
        }
    }
}
