// Switching accounts: Claude Code's login (by hand or automatically) and Claude Desktop's.

use crate::*;

impl App {
    // The tray's two hooks: the state its widget shows, and the switches its buttons ask
    // for (the same calls as the dashboard's buttons).
    #[cfg(windows)]
    pub(crate) fn tray_hooks(self: &Arc<Self>) -> (tray::StateFn, tray::ActFn) {
        let (a, b) = (self.clone(), self.clone());
        let act = move |act: tray::Act| match act {
            tray::Act::Cli(id) => b.activate(&id, "manual"),
            tray::Act::Desktop(uuid) => b.start_desktop_switch(Some(uuid)),
            tray::Act::CheckUpdate => b.check_update(),
            tray::Act::InstallUpdate => b.install_update(),
        };
        (Arc::new(move || a.public_state()), Arc::new(act))
    }

    // Runs a desktop switch in the background: it closes and restarts Claude Desktop,
    // which takes up to a minute. `target` None leaves it signed out, to add an account.
    pub(crate) fn start_desktop_switch(self: &Arc<Self>, target: Option<String>) -> Result<(), String> {
        let dir = self.desktop_dir.clone().ok_or("Claude Desktop's data folder was not found")?;
        if let Some(t) = &target {
            if !self.desktop_store.has_sign_in(t) {
                return Err("No sign-in is saved for this account yet. Sign in to it in Claude Desktop once (Add account).".into());
            }
            if desktop::signed_in(&dir).as_deref() == Some(t.as_str()) {
                return Err("Claude Desktop is signed in to this account already.".into());
            }
        }
        #[cfg(windows)]
        let real = desktop::RealApp { data_dir: dir.clone() };
        #[cfg(windows)]
        if real.inside_claude() {
            return Err("This dashboard was started from inside Claude, so closing Claude would stop it half-way. Start it from Explorer or the Start menu.".into());
        }
        #[cfg(not(windows))]
        return Err("Switching Claude Desktop works on Windows only.".into());
        #[cfg(windows)]
        {
            if self.switching.swap(true, Ordering::SeqCst) {
                return Err("A switch is running already.".into());
            }
            // Every account's Code sessions in its sidebar (docs/specs/desktop-sessions.md).
            let share = (!self.demo && self.st.lock().unwrap().settings.share_desktop_sessions).then(|| self.data_dir().join("desktop-sessions"));
            let app = self.clone();
            thread::spawn(move || {
                let res = desktop::switch(&dir, &app.desktop_store, target.as_deref(), &real, now(), share.as_deref());
                let mut st = app.st.lock().unwrap();
                match res.as_ref().ok().and_then(|s| s.shared.as_ref()) {
                    Some(Ok(n)) if n.any() => app.log(&mut st, format!("Claude Desktop: Code sessions shared ({} added, {} updated, {} removed)", n.added, n.updated, n.removed)),
                    Some(Err(e)) => app.log(&mut st, format!("Claude Desktop: Code sessions not shared: {e}")),
                    _ => {}
                }
                let name = |st: &State, u: &str| st.desktop.iter().find(|d| d.uuid == u).map(|d| d.label.clone()).unwrap_or_else(|| u[..8].to_string());
                let msg = match (&res, &target) {
                    (Ok(_), Some(t)) => format!("Claude Desktop: switched to {}", name(&st, t)),
                    (Ok(_), None) => "Claude Desktop: signed out here; sign in to the account to add".to_string(),
                    (Err(e), _) => format!("Claude Desktop switch failed: {e}"),
                };
                app.log(&mut st, msg);
                app.save(&st);
                app.switching.store(false, Ordering::SeqCst);
            });
            Ok(())
        }
    }

    // ----- switching -----
    // Makes `id` the account Claude Code uses. Takes the state lock itself; call it
    // without holding it.
    pub(crate) fn activate(&self, id: &str, reason: &str) -> Result<(), String> {
        let (creds, uuid) = {
            let st = self.st.lock().unwrap();
            let a = st.accounts.iter().find(|a| a.id == id).ok_or("not found")?;
            (a.creds.clone(), a.account_uuid.clone())
        };
        // A login Anthropic refuses, or another account's, is never handed over.
        let checked = if self.demo { creds } else { self.verify_login(&creds, uuid.as_deref())? };
        let mut st = self.st.lock().unwrap();
        let acc = {
            let a = st.accounts.iter_mut().find(|a| a.id == id).ok_or("not found")?;
            a.creds = checked;
            a.error = None;
            a.clone()
        };
        if !self.demo {
            // Claude Code's refresh and credential-store locks while its login is read
            // and replaced: it can't renew or save its own in between.
            let home = self.creds_file.parent().unwrap_or(Path::new("."));
            let _held = if self.keychain {
                None
            } else {
                let r = lock::refresh(home).ok_or("Claude Code is renewing its login right now; try again in a few seconds")?;
                let s = lock::storage(home).ok_or("Claude Code is saving its login right now; try again in a few seconds")?;
                Some((r, s))
            };
            // Keep the outgoing account's newest tokens first: once the file is
            // overwritten they are gone, and its stored refresh token may be spent.
            if let Some(live) = self.read_live() {
                if !self.note_live(&mut st, &live) {
                    return Err("Claude Code's current login is not identified yet; try again in a few seconds".into());
                }
            }
            self.write_live(&acc.creds)?;
        }
        if !self.demo {
            self.write_oauth_account(&acc);
        }
        let from = st.active_id.as_ref().and_then(|f| st.accounts.iter().find(|a| &a.id == f)).map(|a| a.email.clone());
        st.active_id = Some(acc.id.clone());
        self.log(&mut st, format!("Switched {} → {} ({reason})", from.unwrap_or("(none)".into()), acc.email));
        self.save(&st);
        Ok(())
    }

    // The account automatic switching moves to, with the reason; None to stay.
    pub(crate) fn pick_auto(&self, st: &mut State) -> Option<(String, String)> {
        if !st.settings.auto_switch {
            return None;
        }
        let s = st.settings.clone();
        let active = st.active_id.as_ref().and_then(|id| st.accounts.iter().find(|a| &a.id == id)).cloned()?;
        let which = switch_reason(active.usage.as_ref()?, &s)?;
        let room = |a: &Account| a.usage.as_ref().map(|u| (100 - u.five.u).min(100 - u.week.u)).unwrap_or(0);
        let next = st.accounts.iter()
            .filter(|a| a.enabled && a.id != active.id && a.error.is_none())
            .filter(|a| a.usage.as_ref().is_some_and(|u| has_room(u, &s)))
            .max_by_key(|a| room(a))
            .map(|a| a.id.clone());
        if next.is_none() {
            let msg = format!("{} hit {which} but no other account has room", active.email);
            if st.events.first().map(|e| &e.msg) != Some(&msg) {
                self.log(st, msg);
            }
        }
        next.map(|id| (id, format!("auto: {which}")))
    }
}

// Why the account in use must be left: its 5-hour window at `threshold` or its weekly
// one at `week_threshold`; None while both are under.
pub(crate) fn switch_reason(u: &Usage, s: &Settings) -> Option<String> {
    if u.five.u >= s.threshold {
        Some(format!("5-hour {}%", u.five.u))
    } else if u.week.u >= s.week_threshold {
        Some(format!("weekly {}%", u.week.u))
    } else {
        None
    }
}

// Whether an account can take over: under both thresholds.
pub(crate) fn has_room(u: &Usage, s: &Settings) -> bool {
    switch_reason(u, s).is_none()
}
