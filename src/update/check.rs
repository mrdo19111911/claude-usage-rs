// Asking GitHub for the latest release, and what its answer sets: the status, the log,
// and the saved check time that starts the 3 days (docs/specs/update.md, "When").

use super::*;

impl App {
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
        self.take_answer(latest_release(&self.http), now())
    }

    // GitHub's answer `res`, got at `t`.
    pub(crate) fn take_answer(&self, res: Result<Value, String>, t: i64) -> Result<(), String> {
        let (newer, found) = match &res {
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
                (newer, first.then_some(latest))
            }
            Err(e) => {
                self.set_update(|u| {
                    u.phase = Phase::Error;
                    u.checked_at = Some(t);
                    u.error = Some(e.clone());
                });
                (false, None)
            }
        };
        let mut st = self.st.lock().unwrap();
        // The 3 days start only when nothing waits to be installed: a version found with
        // autoUpdate on is tried again at the next hourly look, and after a restart.
        if res.is_ok() && !(newer && st.settings.auto_update) {
            st.update_checked_at = Some(t);
        }
        if let Some(v) = found {
            self.log(&mut st, format!("Update {v} found"));
        }
        self.save(&st);
        drop(st);
        res.map(|_| ())
    }
}
