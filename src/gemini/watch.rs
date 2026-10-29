// "Copy to add" (docs/specs/gemini.md): while the add dialog is open, the clipboard is
// looked at every 0.5 s. When it holds exactly a Google key (the user pressed Copy next to
// it in AI Studio), the key is checked and saved as POST /api/gemini/key does. Any other
// text is dropped at once: never stored, logged or sent. The clipboard and the save are
// parameters, so tests use neither the real clipboard nor Google.

use crate::*;
use super::key::key_in;

// The watch ends by itself after this, if the dialog is left open.
pub(crate) const WATCH_MS: i64 = 10 * 60_000;
const TICK: Duration = Duration::from_millis(500);

// What reading the clipboard gave.
#[derive(Clone, Debug)]
pub(crate) enum Read {
    Text(String),
    // No text there (a picture, files, nothing).
    Other,
    // Another program has it open: try again on the next tick.
    Busy,
}

pub(crate) trait Source {
    // Changes whenever anything is copied (Windows' clipboard sequence number).
    fn seq(&self) -> u32;
    fn read(&self) -> Read;
}

// In App: null, or what the page shows while the dialog is open.
#[derive(Default)]
pub(crate) struct Watch(Mutex<Inner>);

#[derive(Default)]
struct Inner {
    now: Option<Status>,
    // Which watcher thread is the live one: an off then an on starts a new run, and the
    // old thread, seeing another run, stops.
    run: u64,
}

struct Status {
    state: &'static str,
    error: Option<String>,
    until: i64,
}

impl Watch {
    // Some(run) when a watcher thread is to start; a second on only extends the time.
    pub(crate) fn on(&self, t: i64) -> Option<u64> {
        let mut i = self.0.lock().unwrap();
        if let Some(s) = &mut i.now {
            s.until = t + WATCH_MS;
            return None;
        }
        i.run += 1;
        i.now = Some(Status { state: "waiting", error: None, until: t + WATCH_MS });
        Some(i.run)
    }

    pub(crate) fn off(&self) {
        self.0.lock().unwrap().now = None;
    }

    // /api/state's geminiWatch: null or {state, error, until}.
    pub(crate) fn public(&self) -> Value {
        match &self.0.lock().unwrap().now {
            Some(s) => json!({ "state": s.state, "error": s.error, "until": s.until }),
            None => Value::Null,
        }
    }

    // Whether `run` is the live watch: not closed, timed out or ended by a saved key.
    pub(crate) fn live(&self, run: u64) -> bool {
        let i = self.0.lock().unwrap();
        i.run == run && i.now.is_some()
    }

    // Sets the state while `run` is the live watch (None ends it); false when it is not.
    fn set(&self, run: u64, next: Option<(&'static str, Option<String>)>) -> bool {
        let mut i = self.0.lock().unwrap();
        if i.run != run || i.now.is_none() {
            return false;
        }
        match (next, i.now.as_mut()) {
            (Some((state, error)), Some(s)) => (s.state, s.error) = (state, error),
            _ => i.now = None,
        }
        i.now.is_some()
    }

    // One look at the clipboard at time t. false: the watch is over (stopped, timed out,
    // or a key saved). `seen`: the sequence number last read, so a copy is read only once
    // and a key Google refused is not tried again until something new is copied.
    pub(crate) fn tick(&self, run: u64, t: i64, seen: &mut Option<u32>, src: &impl Source, save: &dyn Fn(&str) -> Result<(), String>) -> bool {
        {
            let mut i = self.0.lock().unwrap();
            match &i.now {
                Some(s) if i.run == run && t >= s.until => {
                    i.now = None;
                    return false;
                }
                Some(_) if i.run == run => {}
                _ => return false,
            }
        }
        let seq = src.seq();
        if *seen == Some(seq) {
            return true;
        }
        let key = match src.read() {
            Read::Busy => return true,
            Read::Other => None,
            Read::Text(text) => {
                let key = key_in(&text);
                wipe(text);
                key
            }
        };
        *seen = Some(seq);
        let Some(key) = key else { return true };
        if !self.set(run, Some(("checking", None))) {
            return false;
        }
        let r = save(&key);
        wipe(key);
        match r {
            Ok(()) => self.set(run, None),
            Err(e) => self.set(run, Some(("error", Some(e)))),
        }
    }
}

// Clipboard text is the user's: zeroed before its memory is given back.
fn wipe(s: String) {
    let mut b = s.into_bytes();
    for x in b.iter_mut() {
        // Volatile, so the compiler keeps a write it would think useless.
        unsafe { std::ptr::write_volatile(x, 0) };
    }
}

// Starts the watcher thread for `run` (from Watch::on): it ends with the watch.
pub(crate) fn start(app: &Arc<App>, run: u64) {
    let app = app.clone();
    thread::spawn(move || {
        let src = real();
        let save = |key: &str| save_for(&app, run, key, &super::upstream::list_models);
        let mut seen = None;
        while app.gemini_watch.tick(run, now(), &mut seen, &src, &save) {
            thread::sleep(TICK);
        }
    });
}

// What the watcher of `run` does with a key: `check` it (with Google, or a test's), save it
// if `run` is still the live watch by then (setup.rs).
pub(super) fn save_for(app: &Arc<App>, run: u64, key: &str, check: super::api::Check) -> Result<(), String> {
    super::api::save_by(app, key, check, super::setup::By::Watch(run))
}

// Tests never read the real clipboard: it may hold anything of the user's.
#[cfg(not(test))]
fn real() -> super::clipboard::System {
    super::clipboard::System
}
