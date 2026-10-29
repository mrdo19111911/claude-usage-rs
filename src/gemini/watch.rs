// "Copy to add" (docs/specs/gemini.md): while the add dialog is open, the clipboard is
// looked at every 0.5 s. When it holds exactly a Google key (the user pressed Copy next to
// it in AI Studio), the key is checked and saved as POST /api/gemini/key does. Any other
// text is dropped at once: never stored, logged or sent. The clipboard and the save are
// parameters, so tests use neither the real clipboard nor Google.

use crate::*;

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

// The key in the copied text, if the text (trimmed) is exactly one: `AIza` + 35 of
// [0-9A-Za-z_-], or `AQ.` + 20-200 of [0-9A-Za-z_.-] (AI Studio's format since May 2026).
pub(crate) fn key_in(text: &str) -> Option<String> {
    let t = text.trim();
    let all = |s: &str, extra: &str| s.chars().all(|c| c.is_ascii_alphanumeric() || extra.contains(c));
    let ok = match (t.strip_prefix("AIza"), t.strip_prefix("AQ.")) {
        (Some(rest), _) => rest.len() == 35 && all(rest, "_-"),
        (_, Some(rest)) => (20..=200).contains(&rest.len()) && all(rest, "_.-"),
        _ => false,
    };
    ok.then(|| t.to_string())
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
        let save = |key: &str| super::api::save(&app, key);
        let mut seen = None;
        while app.gemini_watch.tick(run, now(), &mut seen, &src, &save) {
            thread::sleep(TICK);
        }
    });
}

// Tests never read the real clipboard: it may hold anything of the user's.
#[cfg(not(test))]
fn real() -> super::clipboard::System {
    super::clipboard::System
}

#[cfg(test)]
fn real() -> impl Source {
    struct Quiet;
    impl Source for Quiet {
        fn seq(&self) -> u32 {
            0
        }
        fn read(&self) -> Read {
            Read::Other
        }
    }
    Quiet
}
