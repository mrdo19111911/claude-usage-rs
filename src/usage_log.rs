// The usage log: one line per usage answer, kept 60 days, next to state.json
// (docs/specs/usage-log.md). Its own lock: an append never holds the state's.

use crate::*;
use std::collections::HashMap;
use std::io::Write;

pub(crate) const FILE: &str = "usage-log.jsonl";
pub(crate) const KEEP_MS: i64 = 60 * 24 * HOUR;
// The same figures again are written only this long after the account's last line.
pub(crate) const EVERY_MS: i64 = 10 * 60_000;

#[derive(Default)]
pub(crate) struct Log {
    inner: Mutex<Inner>,
}

#[derive(Default)]
struct Inner {
    // None until started (and in the demo): nothing is written.
    path: Option<PathBuf>,
    // Each account's last line: its time and its figures (the line without t and seed).
    last: HashMap<String, (i64, Value)>,
    lines: u64,
    since: Option<i64>,
    bytes: u64,
    // A failure has been logged: not again.
    failed: bool,
}

impl Log {
    // At start: lines older than 60 days dropped (the file rewritten then), or, with no
    // file yet, seeded from state.json's history; each account's last line remembered.
    pub(crate) fn start(&self, dir: &Path, st: &State, t: i64) -> Result<(), String> {
        let path = dir.join(FILE);
        let mut inner = self.inner.lock().unwrap();
        inner.path = Some(path.clone());
        let (kept, rewrite) = match fs::read_to_string(&path) {
            Ok(text) => {
                let all: Vec<&str> = text.lines().collect();
                let kept: Vec<Value> = all.iter().filter_map(|l| serde_json::from_str::<Value>(l).ok()).filter(|v| v["t"].as_i64().is_some_and(|lt| lt > t - KEEP_MS)).collect();
                let rewrite = kept.len() != all.len() || !(text.is_empty() || text.ends_with('\n'));
                (kept, rewrite)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let seed = seed_lines(st);
                let any = !seed.is_empty();
                (seed, any)
            }
            Err(e) => return Err(format!("Usage log: could not read {}: {e}", path.display())),
        };
        let text: String = kept.iter().map(|v| format!("{v}\n")).collect();
        if rewrite {
            fs::create_dir_all(dir).and_then(|_| write_private(&path, &text)).map_err(|e| format!("Usage log: could not write {}: {e}", path.display()))?;
        }
        inner.lines = kept.len() as u64;
        inner.since = kept.first().and_then(|v| v["t"].as_i64());
        inner.bytes = text.len() as u64;
        for v in kept {
            if let (Some(lt), Some(id)) = (v["t"].as_i64(), v["id"].as_str()) {
                inner.last.insert(id.to_string(), (lt, without_time(&v)));
            }
        }
        Ok(())
    }

    // Appends the account's figures at `t`, unless they are its last line's and that is
    // under 10 minutes old. Ok(true): a line was written.
    pub(crate) fn append(&self, fig: Value, t: i64) -> Result<bool, String> {
        let mut inner = self.inner.lock().unwrap();
        let Some(path) = inner.path.clone() else { return Ok(false) };
        let id = fig["id"].as_str().unwrap_or_default().to_string();
        if inner.last.get(&id).is_some_and(|(lt, f)| *f == fig && t - lt < EVERY_MS) {
            return Ok(false);
        }
        let mut line = json!({ "t": t });
        line.as_object_mut().unwrap().extend(fig.as_object().cloned().unwrap_or_default());
        let text = format!("{line}\n");
        append_to(&path, &text).map_err(|e| format!("Usage log: could not write {}: {e}", path.display()))?;
        inner.lines += 1;
        inner.since.get_or_insert(t);
        inner.bytes += text.len() as u64;
        inner.last.insert(id, (t, fig));
        Ok(true)
    }

    // For /api/state: {"lines", "since", "bytes"}.
    pub(crate) fn stats(&self) -> Value {
        let inner = self.inner.lock().unwrap();
        json!({ "lines": inner.lines, "since": inner.since, "bytes": inner.bytes })
    }

    pub(crate) fn path(&self) -> Option<PathBuf> {
        self.inner.lock().unwrap().path.clone()
    }

    // True the first time only: a failure is logged once.
    fn first_failure(&self) -> bool {
        !std::mem::replace(&mut self.inner.lock().unwrap().failed, true)
    }
}

fn append_to(path: &Path, text: &str) -> std::io::Result<()> {
    let mut o = fs::OpenOptions::new();
    o.append(true).create(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        o.mode(0o600);
    }
    o.open(path)?.write_all(text.as_bytes())
}

fn without_time(v: &Value) -> Value {
    let mut f = v.clone();
    if let Some(o) = f.as_object_mut() {
        o.remove("t");
        o.remove("seed");
    }
    f
}

// An account's line, without its time: the app id, never the email, a token or `raw`.
// None while it has no usage.
pub(crate) fn figures(a: &Account, active: bool) -> Option<Value> {
    let u = a.usage.as_ref()?;
    Some(json!({ "id": a.id, "plan": a.plan, "active": active, "five": u.five, "week": u.week, "scoped": u.scoped }))
}

// state.json's 8 days of (time, 5h %, week %), as lines marked seed, in time order.
fn seed_lines(st: &State) -> Vec<Value> {
    let mut out: Vec<Value> = st.accounts.iter().flat_map(|a| a.history.iter().map(move |&(t, five, week)| json!({
        "t": t, "id": a.id, "plan": null, "active": false,
        "five": { "u": five, "resetsAt": null }, "week": { "u": week, "resetsAt": null },
        "scoped": [], "seed": true,
    }))).collect();
    out.sort_by_key(|v| v["t"].as_i64());
    out
}

impl App {
    // At start, under the state's lock (main). Not in the demo.
    pub(crate) fn start_usage_log(&self, st: &mut State) {
        if self.demo {
            return;
        }
        if let Err(e) = self.usage_log.start(self.data_dir(), st, now()) {
            if self.usage_log.first_failure() {
                self.log(st, e);
            }
        }
    }

    // After a usage answer (poll.rs), outside the state's lock; a failure is logged once
    // and polling goes on.
    pub(crate) fn log_usage(&self, fig: Value) {
        if let Err(e) = self.usage_log.append(fig, now()) {
            if self.usage_log.first_failure() {
                let mut st = self.st.lock().unwrap();
                self.log(&mut st, e);
            }
        }
    }
}
