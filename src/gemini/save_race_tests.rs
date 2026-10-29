// Saves that overlap: a watch closed while Google is asked, two keys saved at once. Each
// race is held at one step by a Gate, so it runs in the same order every time.

use super::api::save;
use super::setup::MID_SETUP;
use super::profile::profile_dir;
use super::script::script_path;
use super::test_app::{app, google_ok, temp_dir, Gate, KEY_A, KEY_B};
use super::watch::{save_for, Read, Source};
use crate::App;
use std::sync::{mpsc, Arc};
use std::time::Duration;

// A clipboard holding one copy that never changes.
struct Copied(&'static str);

impl Source for Copied {
    fn seq(&self) -> u32 {
        1
    }
    fn read(&self) -> Read {
        Read::Text(self.0.into())
    }
}

// The key in state.json, and whether cgemini.cmd and the profile carry its token.
fn saved(a: &App) -> Option<(String, bool)> {
    let c = a.st.lock().unwrap().gemini.clone()?;
    let script = std::fs::read_to_string(script_path(a)).unwrap_or_default();
    let settings = std::fs::read_to_string(profile_dir(a).join("settings.json")).unwrap_or_default();
    let same = script.contains(&format!("x-cud-token: {}", c.token)) && settings.contains(&format!("\"{}\"", c.token));
    Some((c.key, same))
}

// The watcher of `run` sees KEY_A; Google's answer is held at `gate`. Returns what tick says.
fn watch_held(a: &Arc<App>, run: u64, gate: &Gate) -> bool {
    let held = |k: &str| {
        gate.pass();
        google_ok(k)
    };
    let mut seen = None;
    a.gemini_watch.tick(run, 1, &mut seen, &Copied(KEY_A), &|k: &str| save_for(a, run, k, &held))
}

#[test]
fn a_watch_closed_and_opened_again_during_the_check_saves_nothing() {
    let a = app(&temp_dir("race-reopen"), 1, None);
    let run = a.gemini_watch.on(0).unwrap();
    let gate = Gate::default();
    std::thread::scope(|s| {
        let old = s.spawn(|| watch_held(&a, run, &gate));
        gate.reached();
        a.gemini_watch.off(); // the dialog closed...
        assert!(a.gemini_watch.on(2).is_some(), "...and opened again: a new watch");
        gate.open();
        assert!(!old.join().unwrap(), "the old watcher stops");
    });
    assert!(saved(&a).is_none(), "the key of a closed watch is not saved");
    assert!(!script_path(&a).exists(), "nor cgemini.cmd written");
    assert_eq!(a.gemini_watch.public()["state"], "waiting", "the new watch goes on");
}

#[test]
fn a_key_pasted_during_the_watch_check_is_kept() {
    let a = app(&temp_dir("race-pasted"), 1, None);
    let run = a.gemini_watch.on(0).unwrap();
    let gate = Gate::default();
    std::thread::scope(|s| {
        let old = s.spawn(|| watch_held(&a, run, &gate));
        gate.reached();
        save(&a, KEY_B, &google_ok).unwrap(); // pasted by hand meanwhile
        gate.open();
        assert!(!old.join().unwrap(), "the watch is over");
    });
    assert_eq!(saved(&a), Some((KEY_B.to_string(), true)), "the pasted key stays, with its own token");
    assert_eq!(a.gemini_watch.public(), serde_json::Value::Null);
}

#[test]
fn two_saves_at_once_leave_one_token_everywhere() {
    let a = app(&temp_dir("race-two"), 1, None);
    let gate = Arc::new(Gate::default());
    let a = &a;
    std::thread::scope(|s| {
        let g = gate.clone();
        let first = s.spawn(move || {
            MID_SETUP.with(|h| *h.borrow_mut() = Some(Box::new(move || g.pass())));
            save(&a, KEY_A, &google_ok)
        });
        gate.reached(); // the first save is in the middle of its setup
        let (done, second_done) = mpsc::channel();
        let second = s.spawn(move || {
            let r = save(&a, KEY_B, &google_ok);
            let _ = done.send(());
            r
        });
        // Without one setup at a time the second is done at once; with it, it waits.
        let _ = second_done.recv_timeout(Duration::from_millis(300));
        gate.open();
        first.join().unwrap().unwrap();
        second.join().unwrap().unwrap();
    });
    let (key, same) = saved(&a).unwrap();
    assert!(same, "state.json, cgemini.cmd and the profile have the same token (key {key})");
}
