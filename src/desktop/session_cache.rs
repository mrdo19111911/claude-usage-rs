// The session of a saved cookie store, read only when its file changed (spec:
// docs/specs/desktop-signed-out.md, "When a saved store is read").
//
// The state is asked for every 2 seconds; a saved store changes only when a profile is
// saved. So what a store holds is kept per file path with the file's size and modified
// time (and its journal's), and the store is read again only when those differ.
// In: the path of a saved `Cookies` file and the time. Out: as `read_session`.

use super::cookie_session::{read_session, Session};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

/// Size and modified time of a file (None: no such file).
type Stamp = Option<(u64, SystemTime)>;

struct Held {
    /// Of the store and of its journal when it was read.
    stamps: (Stamp, Stamp),
    /// Its session as of 1970: `Live(Some(t))` keeps the latest expiry, passed or not.
    session: Session,
}

static HELD: Mutex<BTreeMap<PathBuf, Held>> = Mutex::new(BTreeMap::new());

/// The session `cookies` holds at `now_ms` (None: no file, or not a cookie store).
pub(super) fn saved_session(cookies: &Path, now_ms: i64) -> Option<Session> {
    let mut held = HELD.lock().unwrap_or_else(|e| e.into_inner());
    let stamps = (stamp(cookies), stamp(&journal(cookies)));
    if stamps.0.is_none() {
        held.remove(cookies);
        return None;
    }
    if let Some(h) = held.get(cookies).filter(|h| h.stamps == stamps) {
        return Some(at(h.session, now_ms));
    }
    count(cookies);
    // Stamped before the read: a file written meanwhile differs next time and is read again.
    match read_session(cookies, 0) {
        Some(session) => {
            held.insert(cookies.to_path_buf(), Held { stamps, session });
            Some(at(session, now_ms))
        }
        None => {
            held.remove(cookies);
            None
        }
    }
}

/// What a session read as of 1970 is at `now_ms`: a passed expiry has ended.
fn at(session: Session, now_ms: i64) -> Session {
    match session {
        Session::Live(Some(until)) if until <= now_ms => Session::Ended,
        s => s,
    }
}

fn journal(cookies: &Path) -> PathBuf {
    let mut name = cookies.file_name().unwrap_or_default().to_os_string();
    name.push("-journal");
    cookies.with_file_name(name)
}

fn stamp(file: &Path) -> Stamp {
    let m = fs::metadata(file).ok().filter(|m| m.is_file())?;
    Some((m.len(), m.modified().ok()?))
}

// One read of `cookies` from the disk (counted in tests only).
fn count(_cookies: &Path) {
}

// How many times `cookies` was read from the disk. A plain comment, not a doc one: the
// public build strips the test-only helper below, so a /// here would document nothing.
