// Claude Desktop's Code session cards shared between accounts (docs/specs/desktop-sessions.md).
//
// Desktop lists Code sessions per account from
// `claude-code-sessions/<account>/<org>/local_*.json` ("cards"). Run inside a Desktop
// switch while the app is closed, so every folder can be written. Rules from
// bdarbaz/claude-session-sync (MIT): Syncer.run.

use super::sessions_io::{backup, trash, write_card, write_index};
pub(super) use super::sessions_io::SyncState;
use super::sessions_rules::{fresh_copy, freshness, merged, templates};
use super::subdirs;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

pub(super) type Card = Map<String, Value>;

/// What a sync changed, for the event log.
#[derive(Default, Debug, PartialEq)]
pub struct Counts {
    pub added: usize,
    pub updated: usize,
    pub removed: usize,
}

impl Counts {
    pub fn any(&self) -> bool {
        self.added + self.updated + self.removed > 0
    }
}

/// One `<account>/<org>` folder as read before the sync.
pub(super) struct Folder {
    pub(super) key: String,
    pub(super) account: String,
    pub(super) path: PathBuf,
    /// Cards that are JSON objects with a string sessionId, by file name.
    pub(super) cards: BTreeMap<String, Card>,
    /// Every local_*.json there, readable or not.
    pub(super) present: BTreeSet<String>,
    pub(super) readable: bool,
}

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
    let mut f = Folder { key: format!("{account}/{org}"), account: account.into(), path, cards: BTreeMap::new(), present: BTreeSet::new(), readable: false };
    let Ok(Ok(entries)) = fs::read_dir(&f.path).map(|rd| rd.collect::<Result<Vec<_>, _>>()) else { return f };
    f.readable = true;
    for e in entries {
        let name = e.file_name().to_string_lossy().into_owned();
        if is_card_name(&name) {
            if let Some(c) = read_card(&e.path()) {
                f.cards.insert(name.clone(), c);
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

/// Shares the cards of `<desktop dir>/claude-code-sessions` between its folders; the
/// sync's state, trash and backup live in `state_dir`. Desktop must be closed.
pub fn sync(desktop_dir: &Path, state_dir: &Path, stamp: i64) -> Result<Counts, String> {
    let root = desktop_dir.join("claude-code-sessions");
    let mut folders = load(&root);
    if folders.len() < 2 {
        return Ok(Counts::default());
    }
    let mut st = SyncState::load(state_dir);
    if !st.backed_up {
        backup(&root, state_dir, stamp)?;
        st.backed_up = true;
        st.save(state_dir)?;
    }
    let res = run(&mut folders, &mut st, state_dir, stamp);
    st.save(state_dir)?;
    res
}

/// One pass: removals, then adds and updates, then the archive indexes. A failed write
/// is reported after the others were tried.
pub(super) fn run(folders: &mut [Folder], st: &mut SyncState, state_dir: &Path, stamp: i64) -> Result<Counts, String> {
    let (mut n, mut errs) = (Counts::default(), vec![]);
    let templates = templates(folders, &st.copied);
    // 1. A card the user deleted in one folder since the last sync: to trash elsewhere.
    // A folder that cannot be read is not "everything deleted".
    for i in 0..folders.len() {
        let seen = st.seen.get(&folders[i].key).cloned().unwrap_or_default();
        let gone: Vec<String> = seen.into_iter().filter(|s| folders[i].readable && !folders[i].present.contains(s)).collect();
        for name in gone {
            for g in folders.iter_mut().filter(|g| g.readable && g.cards.contains_key(&name)) {
                match trash(g, &name, state_dir, stamp) {
                    Ok(()) => {
                        g.cards.remove(&name);
                        g.present.remove(&name);
                        n.removed += 1;
                    }
                    Err(e) => errs.push(e),
                }
            }
        }
    }
    // 2. Every folder gets every card; a less fresh one is brought up to the freshest.
    let names: BTreeSet<String> = folders.iter().flat_map(|f| f.cards.keys().cloned()).collect();
    for name in names {
        let rank = |f: &Folder| f.cards.get(&name).map(|c| (freshness(c), f.key.clone()));
        let Some(bi) = (0..folders.len()).filter(|&i| rank(&folders[i]).is_some()).max_by(|&a, &b| {
            let (x, y) = (rank(&folders[a]).unwrap_or_default(), rank(&folders[b]).unwrap_or_default());
            x.0.total_cmp(&y.0).then(x.1.cmp(&y.1))
        }) else {
            continue;
        };
        let best = folders[bi].cards[&name].clone();
        for (j, f) in folders.iter_mut().enumerate().filter(|(j, f)| *j != bi && f.readable) {
            let new = match f.cards.get(&name) {
                // An unreadable card of that name stays as it is.
                None if f.present.contains(&name) => continue,
                None => fresh_copy(&best, templates[j].as_ref()),
                Some(cur) if freshness(cur) < freshness(&best) => merged(&best, cur),
                Some(_) => continue,
            };
            if let Err(e) = write_card(f, &name, &new) {
                errs.push(e);
                continue;
            }
            if f.cards.insert(name.clone(), new).is_some() {
                n.updated += 1;
            } else {
                n.added += 1;
                f.present.insert(name.clone());
                let copied = st.copied.entry(f.key.clone()).or_default();
                if !copied.contains(&name) {
                    copied.push(name.clone());
                }
            }
        }
    }
    // 3. The archive index from each card's isArchived; 4. what the next sync compares to.
    for f in folders.iter().filter(|f| f.readable) {
        let want = f.cards.values().filter(|c| c.get("isArchived") == Some(&Value::Bool(true))).filter_map(|c| c["sessionId"].as_str().map(str::to_string)).collect();
        if let Err(e) = write_index(f, &want) {
            errs.push(e);
        }
        st.seen.insert(f.key.clone(), f.cards.keys().cloned().collect());
        if let Some(c) = st.copied.get_mut(&f.key) {
            c.retain(|x| f.present.contains(x));
            c.sort();
        }
    }
    match errs.first() {
        None => Ok(n),
        Some(e) => Err(format!("{e} ({} of the writes failed)", errs.len())),
    }
}
