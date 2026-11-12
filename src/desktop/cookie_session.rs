// Whether a Chromium cookie store holds a live claude.ai session (spec:
// docs/specs/desktop-signed-out.md, prior art: docs/SO_SANH_DESKTOP_SIGNED_OUT.md).
//
// Claude Desktop shows its sign-in screen when its store has no `sessionKey` cookie
// for claude.ai, whatever config.json holds. The store is read from a copy (with its
// journal, so a half-written change is rolled back there), like browser_cookie3 does;
// the query follows rookie's (rookie-rs/src/browser/chromium.rs).

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Microseconds from 1601-01-01 to 1970-01-01: Chromium's clock starts at 1601.
const EPOCH_GAP_US: i64 = 11_644_473_600_000_000;

/// Unix milliseconds as Chromium writes `expires_utc`.
pub(crate) fn chromium_time(unix_ms: i64) -> i64 {
    unix_ms * 1000 + EPOCH_GAP_US
}

// How many live sessionKey rows, and their earliest and latest expiry (0: none set).
const LIVE_SESSION: &str = "SELECT COUNT(*), MIN(expires_utc), MAX(expires_utc) FROM cookies \
     WHERE name = 'sessionKey' AND (host_key = 'claude.ai' OR host_key LIKE '%.claude.ai') \
     AND (expires_utc = 0 OR expires_utc > ?1)";

/// What a cookie store holds of claude.ai's session.
#[derive(Debug, PartialEq)]
pub enum Session {
    /// Live until then (Unix ms); None: a cookie without expiry.
    Live(Option<i64>),
    Ended,
}

/// The store's session, or None when it is missing, locked or not a cookie store.
pub(crate) fn read_session(cookies: &Path, now_ms: i64) -> Option<Session> {
    let copy = Copy::of(cookies)?;
    let db = rusqlite::Connection::open_with_flags(copy.db(), rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE).ok()?;
    let row = |r: &rusqlite::Row| Ok((r.get::<_, i64>(0)?, r.get::<_, Option<i64>>(1)?, r.get::<_, Option<i64>>(2)?));
    let (n, min, max) = db.query_row(LIVE_SESSION, [chromium_time(now_ms)], row).ok()?;
    Some(match (n, min, max) {
        (0, _, _) => Session::Ended,
        (_, Some(0), _) => Session::Live(None),
        (_, _, max) => Session::Live(max.map(|us| (us - EPOCH_GAP_US) / 1000)),
    })
}

/// Some(true): a live claude.ai sessionKey; Some(false): the store reads and has none;
/// None: no file, or not a cookie store (unknown).
pub(crate) fn session_state(cookies: &Path, now_ms: i64) -> Option<bool> {
    read_session(cookies, now_ms).map(|s| s != Session::Ended)
}

/// A private copy of a store and its journal, removed when dropped.
struct Copy(PathBuf);

impl Copy {
    fn of(cookies: &Path) -> Option<Copy> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        if !cookies.is_file() {
            return None;
        }
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("cud-cookies-{}-{n}", std::process::id()));
        let copy = Copy(dir);
        fs::create_dir_all(&copy.0).ok()?;
        fs::copy(cookies, copy.db()).ok()?;
        let journal = cookies.with_file_name(format!("{}-journal", cookies.file_name()?.to_string_lossy()));
        if journal.is_file() {
            fs::copy(&journal, copy.0.join("Cookies-journal")).ok()?;
        }
        Some(copy)
    }

    fn db(&self) -> PathBuf {
        self.0.join("Cookies")
    }
}

impl Drop for Copy {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
