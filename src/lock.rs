// Claude Code's own file locks, taken the way it takes them (the npm package
// proper-lockfile): a directory next to the file, created with mkdir, kept fresh by
// touching its mtime, and counted as abandoned once older than `stale`. Holding them
// keeps a running Claude Code from renewing or saving its login while we read and
// write it. Names and stale times checked in Claude Code 2.1.310 (and as
// claude-usage-monitor's live.py takes them).

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime};

const TOUCH_EVERY: Duration = Duration::from_secs(2);

pub struct DirLock {
    held: Vec<PathBuf>,
    stop: Arc<AtomicBool>,
    toucher: Option<thread::JoinHandle<()>>,
}

/// Takes one lock directory; one older than `stale_ms` (its holder died) is taken over.
fn take_dir(d: &Path, stale_ms: u64) -> bool {
    for _ in 0..2 {
        match fs::create_dir(d) {
            Ok(()) => return true,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(_) => return false,
        }
        let age = match fs::metadata(d).and_then(|m| m.modified()) {
            Ok(t) => SystemTime::now().duration_since(t).unwrap_or_default(),
            Err(_) => continue, // released meanwhile
        };
        if age < Duration::from_millis(stale_ms) || fs::remove_dir(d).is_err() {
            return false;
        }
    }
    false
}

fn touch(d: &Path) {
    let mut o = fs::OpenOptions::new();
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        // FILE_WRITE_ATTRIBUTES, and FILE_FLAG_BACKUP_SEMANTICS to open a directory.
        o.access_mode(0x100).custom_flags(0x0200_0000);
    }
    #[cfg(not(windows))]
    o.read(true);
    if let Ok(f) = o.open(d) {
        let _ = f.set_modified(SystemTime::now());
    }
}

impl DirLock {
    /// All of `dirs` or none, `attempts` tries about `wait` apart.
    pub fn acquire(dirs: &[PathBuf], stale_ms: u64, attempts: u32, wait: Duration) -> Option<DirLock> {
        for i in 0..attempts {
            if i > 0 {
                // Up to twice `wait`, so two waiters don't retry in step.
                let jitter = (SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().subsec_nanos() % 1000) as u64;
                thread::sleep(wait + wait.mul_f64(jitter as f64 / 1000.0));
            }
            let mut held = vec![];
            for d in dirs {
                if !take_dir(d, stale_ms) {
                    break;
                }
                held.push(d.clone());
            }
            if held.len() == dirs.len() {
                let stop = Arc::new(AtomicBool::new(false));
                let (s, h) = (stop.clone(), held.clone());
                // Naps of 200 ms so a release never waits long; a touch every TOUCH_EVERY.
                let toucher = thread::spawn(move || {
                    let mut slept = Duration::ZERO;
                    while !s.load(Ordering::Relaxed) {
                        thread::sleep(Duration::from_millis(200));
                        slept += Duration::from_millis(200);
                        if slept >= TOUCH_EVERY && !s.load(Ordering::Relaxed) {
                            slept = Duration::ZERO;
                            h.iter().for_each(|d| touch(d));
                        }
                    }
                });
                return Some(DirLock { held, stop, toucher: Some(toucher) });
            }
            for d in held.iter().rev() {
                let _ = fs::remove_dir(d);
            }
        }
        None
    }
}

impl Drop for DirLock {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.toucher.take() {
            let _ = t.join();
        }
        for d in self.held.iter().rev() {
            let _ = fs::remove_dir(d);
        }
    }
}

/// Claude Code's OAuth refresh lock: held around a whole token renewal.
pub fn refresh(home: &Path) -> Option<DirLock> {
    let legacy = fs::canonicalize(home).unwrap_or_else(|_| home.to_path_buf());
    let mut legacy = legacy.into_os_string();
    legacy.push(".lock");
    DirLock::acquire(&[home.join(".oauth_refresh.lock"), PathBuf::from(legacy)], 60_000, 10, Duration::from_millis(500))
}

/// Claude Code's credential-store lock: every read-modify-write of .credentials.json.
pub fn storage(home: &Path) -> Option<DirLock> {
    DirLock::acquire(&[home.join(".storage-write.lock")], 15_000, 10, Duration::from_millis(300))
}

/// Claude Code's lock on its config file (~/.claude.json.lock).
pub fn config(file: &Path) -> Option<DirLock> {
    let mut p = file.as_os_str().to_owned();
    p.push(".lock");
    DirLock::acquire(&[PathBuf::from(p)], 10_000, 10, Duration::from_millis(300))
}
