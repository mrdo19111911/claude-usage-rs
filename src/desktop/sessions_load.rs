// Reading Claude Desktop's Code session folders for the sync (docs/specs/desktop-sessions.md).

use super::sessions::{Card, Folder};
use super::subdirs;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

fn is_uuid(s: &str) -> bool {
    s.len() == 36 && s.bytes().enumerate().all(|(i, b)| if [8, 13, 18, 23].contains(&i) { b == b'-' } else { b.is_ascii_hexdigit() })
}

fn is_card_name(s: &str) -> bool {
    let mid = s.strip_prefix("local_").and_then(|r| r.strip_suffix(".json")).unwrap_or("");
    !mid.is_empty() && mid.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

fn read_card(p: &Path) -> Option<Card> {
    match serde_json::from_str(&fs::read_to_string(p).ok()?) {
        Ok(Value::Object(m)) if m.get("sessionId").is_some_and(Value::is_string) => Some(m),
        _ => None,
    }
}

fn read_folder(account: &str, org: &str, path: PathBuf) -> Folder {
    let mut f = Folder { key: format!("{account}/{org}"), account: account.into(), path, cards: BTreeMap::new(), present: BTreeSet::new(), mtimes: BTreeMap::new(), readable: false };
    let Ok(Ok(entries)) = fs::read_dir(&f.path).map(|rd| rd.collect::<Result<Vec<_>, _>>()) else { return f };
    f.readable = true;
    for e in entries {
        let name = e.file_name().to_string_lossy().into_owned();
        if is_card_name(&name) {
            if let Some(c) = read_card(&e.path()) {
                f.cards.insert(name.clone(), c);
            }
            if let Some(t) = e.metadata().ok().and_then(|m| m.modified().ok()).and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()) {
                f.mtimes.insert(name.clone(), t.as_millis() as i64);
            }
            f.present.insert(name);
        }
    }
    f
}

/// The `<account>/<org>` folders under `root` whose names are both UUIDs.
pub(super) fn load(root: &Path) -> Vec<Folder> {
    let name = |p: &Path| p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let mut v = vec![];
    for acc in subdirs(root, "").into_iter().filter(|p| is_uuid(&name(p))) {
        for org in subdirs(&acc, "").into_iter().filter(|p| is_uuid(&name(p))) {
            v.push(read_folder(&name(&acc), &name(&org), org));
        }
    }
    v
}
