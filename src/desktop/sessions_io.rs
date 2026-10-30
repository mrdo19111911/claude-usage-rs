// Files of the session sync (docs/specs/desktop-sessions.md): writes through a temp file,
// the trash, the first backup and the sync's own state.

use super::sessions::{Card, Folder};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

pub(super) const INDEX: &str = "archived-sessions.idx";

/// `<data dir>/desktop-sessions/state.json`: the cards seen in each folder at the last
/// sync, the ones this app copied there, and whether the first backup was made.
#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(super) struct SyncState {
    #[serde(default)]
    pub(super) seen: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub(super) copied: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub(super) backed_up: bool,
}

impl SyncState {
    /// Missing or unreadable: a new one (it deletes nothing and backs up again).
    pub(super) fn load(dir: &Path) -> SyncState {
        fs::read_to_string(dir.join("state.json")).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
    }

    pub(super) fn save(&self, dir: &Path) -> Result<(), String> {
        fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        write_atomic(&dir.join("state.json"), &serde_json::to_string_pretty(self).unwrap_or_default())
    }
}

/// Writes through a temp file in the same folder, then renames it over `path`. Not named
/// local_*.json.tmp: the app treats those as its own.
pub(super) fn write_atomic(path: &Path, text: &str) -> Result<(), String> {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let tmp = path.with_file_name(format!(".cud-sync-{name}.tmp"));
    let res = fs::write(&tmp, text).and_then(|_| fs::rename(&tmp, path));
    if res.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    res.map_err(|e| format!("{}: {e}", path.display()))
}

pub(super) fn write_card(f: &Folder, name: &str, card: &Card) -> Result<(), String> {
    write_atomic(&f.path.join(name), &serde_json::to_string(card).unwrap_or_default())
}

/// The folder's archive index set to `want`, written only when it differs.
pub(super) fn write_index(f: &Folder, want: &BTreeSet<String>) -> Result<(), String> {
    let path = f.path.join(INDEX);
    let current: Option<Vec<String>> = fs::read_to_string(&path)
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .and_then(|v| v["archived"].as_array().map(|a| a.iter().map(|x| x.as_str().map(str::to_string)).collect::<Option<Vec<_>>>()))
        .flatten();
    let same = match current {
        Some(mut c) => {
            c.sort();
            c.into_iter().eq(want.iter().cloned())
        }
        None => want.is_empty() && !path.exists(),
    };
    if same {
        return Ok(());
    }
    write_atomic(&path, &json!({ "v": 1, "archived": want }).to_string())
}

/// Moves a card to `<state>/trash/<account8>-<org8>/<name>.<ms>`; never deletes it.
pub(super) fn trash(f: &Folder, name: &str, state_dir: &Path, stamp: i64) -> Result<(), String> {
    let (acc, org) = f.key.split_once('/').unwrap_or((&f.key, ""));
    let dir = state_dir.join("trash").join(format!("{}-{}", &acc[..8.min(acc.len())], &org[..8.min(org.len())]));
    fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let (src, dst) = (f.path.join(name), dir.join(format!("{name}.{stamp}")));
    if fs::rename(&src, &dst).is_err() {
        // Another drive: copy, then remove the original only once the copy is there.
        fs::copy(&src, &dst).and_then(|_| fs::remove_file(&src)).map_err(|e| format!("{}: {e}", src.display()))?;
    }
    Ok(())
}

/// The whole claude-code-sessions folder copied to `<state>/backup-<yyyymmdd-hhmmss>/`
/// (UTC).
pub(super) fn backup(root: &Path, state_dir: &Path, stamp: i64) -> Result<(), String> {
    let dst = state_dir.join(format!("backup-{}", utc_stamp(stamp)));
    copy_tree(root, &dst).map_err(|e| format!("backup: {e}"))
}

fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::create_dir_all(to)?;
    for e in fs::read_dir(from)? {
        let e = e?;
        let (src, dst) = (e.path(), to.join(e.file_name()));
        if e.file_type()?.is_dir() {
            copy_tree(&src, &dst)?;
        } else {
            fs::copy(&src, &dst)?;
        }
    }
    Ok(())
}

/// epoch ms -> "yyyymmdd-hhmmss" (UTC).
pub(super) fn utc_stamp(ms: i64) -> String {
    let (days, secs) = (ms.div_euclid(86_400_000), ms.div_euclid(1000).rem_euclid(86_400));
    // Days to a civil date (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}{m:02}{d:02}-{:02}{:02}{:02}", secs / 3600, secs / 60 % 60, secs % 60)
}
