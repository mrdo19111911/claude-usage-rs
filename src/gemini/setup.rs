// Saving a checked key (docs/specs/gemini.md, "Onboarding"): cgemini.cmd, its profile and
// PATH first, state.json last, so the page never hears "ready" before cgemini is; a step
// that fails puts the files back and saves nothing. One setup at a time: two saves at once
// (a copied key and a pasted one) would leave one token in state.json and the other in
// cgemini.cmd, and the proxy would answer cgemini 401.

use crate::*;
use super::{path, script, Config};
use std::sync::MutexGuard;

// Held while state.json's key and the files carrying its token change (save, model,
// remove, Use this). For the whole process, not one App: the PATH it edits is the user's.
pub(super) fn lock() -> MutexGuard<'static, ()> {
    static ONE: Mutex<()> = Mutex::new(());
    // A panic in another setup leaves nothing half-locked to protect: go on.
    ONE.lock().unwrap_or_else(|e| e.into_inner())
}

// Who saves: the page (any copy-to-add watch ends: nothing to wait for), or the watch of
// `run`, only while it is still the live one (not closed, nor another key saved, meanwhile).
pub(super) enum By {
    Page,
    Watch(u64),
}

pub(super) fn commit(app: &Arc<App>, cfg: Config, by: By) -> Result<(), String> {
    let _one = lock();
    // Under the lock: a close before this drops the key; one after it waits for the save.
    if let By::Watch(run) = by {
        if !app.gemini_watch.live(run) {
            return Err("The window was closed: the key was not saved".into());
        }
    }
    let before = app.st.lock().unwrap().gemini.clone();
    if let Err(e) = files(app, &cfg) {
        undo(app, before.as_ref());
        return Err(e);
    }
    {
        let mut st = app.st.lock().unwrap();
        st.gemini = Some(cfg);
        let end = super::public(&st)["keyEnd"].as_str().unwrap_or("").to_string();
        app.log(&mut st, format!("Gemini: key …{end} saved; type cgemini in a new terminal"));
        app.save(&st);
    }
    // A watch ends only its own run (watch.rs), never a newer one.
    if let By::Page = by {
        app.gemini_watch.off();
    }
    Ok(())
}

fn files(app: &App, cfg: &Config) -> Result<(), String> {
    script::write(app, cfg).map_err(|e| format!("cgemini.cmd could not be written: {e}"))?;
    path::add(&script::bin_dir(app)).map_err(|e| format!("The cgemini folder could not be put on PATH: {e}"))
}

// A setup that failed: cgemini.cmd and the profile back to the key saved before, or the
// script away when there was none (its profile's token then opens nothing).
fn undo(app: &App, before: Option<&Config>) {
    match before {
        Some(c) => {
            let _ = script::write(app, c);
        }
        None => script::remove(app),
    }
}

// Tests: what a setup does once it holds the lock (hold it there), on this thread.
