// Audit log: one snapshot line per poll cycle, next to state.json (docs/specs/audit-log.md).
// Unlike usage-log (per-account figures, `active` = Code CLI only), this records who was in
// use — Code AND Claude Desktop — and each account's figures/error, so a missed auto-switch
// is reconstructable. Its own file; appends are best-effort and never fail a poll.

use crate::*;

pub(crate) const FILE: &str = "audit.jsonl";
pub(crate) const KEEP_MS: i64 = 60 * 24 * HOUR;

// One poll's snapshot. Pure (no I/O): what the poll saw after mark_in_use, before switching.
pub(crate) fn snapshot(st: &State, desk: Option<&str>, t: i64) -> Value {
    let accounts: Vec<Value> = st
        .accounts
        .iter()
        .map(|a| {
            json!({
                "id": a.id,
                "inUse": a.in_use,
                "error": a.error,
                "five": a.usage.as_ref().map(|u| u.five.u),
                "week": a.usage.as_ref().map(|u| u.week.u),
            })
        })
        .collect();
    json!({
        "t": t,
        "codeActive": st.active_id,
        "desktop": desk,
        "accounts": accounts,
    })
}

#[derive(Default)]
pub(crate) struct Audit {
    // None until started (and in the demo): nothing is written.
    path: Mutex<Option<PathBuf>>,
}

impl Audit {
    // At start: drop lines older than 60 days (rewrite the file), then remember the path.
    pub(crate) fn start(&self, dir: &Path, t: i64) -> std::io::Result<()> {
        let path = dir.join(FILE);
        if let Ok(text) = fs::read_to_string(&path) {
            let kept: String = text
                .lines()
                .filter(|l| {
                    serde_json::from_str::<Value>(l)
                        .ok()
                        .and_then(|v| v["t"].as_i64())
                        .is_some_and(|lt| lt > t - KEEP_MS)
                })
                .map(|l| format!("{l}\n"))
                .collect();
            if kept.len() != text.len() {
                fs::create_dir_all(dir)?;
                write_private(&path, &kept)?;
            }
        }
        *self.path.lock().unwrap() = Some(path);
        Ok(())
    }

    // Appends one snapshot line. Best-effort: a write error is dropped, polling goes on.
    pub(crate) fn append(&self, line: Value) {
        let Some(path) = self.path.lock().unwrap().clone() else { return };
        let mut o = fs::OpenOptions::new();
        o.append(true).create(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            o.mode(0o600);
        }
        if let Ok(mut f) = o.open(&path) {
            use std::io::Write;
            let _ = f.write_all(format!("{line}\n").as_bytes());
        }
    }

    pub(crate) fn path(&self) -> Option<PathBuf> {
        self.path.lock().unwrap().clone()
    }
}
