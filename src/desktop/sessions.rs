// Claude Desktop's Code session cards shared between accounts (docs/specs/desktop-sessions.md).
//
// Desktop lists Code sessions per account from
// `claude-code-sessions/<account>/<org>/local_*.json` ("cards"). Run inside a Desktop
// switch while the app is closed, so every folder can be written. Rules from
// bdarbaz/claude-session-sync (MIT): Syncer.run.

use super::sessions_io::{backup, trash, write_card, write_index};
pub(super) use super::sessions_load::load;
pub(super) use super::sessions_io::SyncState;
use super::sessions_rules::{fresh_copy, freshness, mark, merged, templates};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
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
    /// When each card file was last written (ms), to break ties between equal times.
    pub(super) mtimes: BTreeMap<String, i64>,
    pub(super) readable: bool,
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
    // Also before the first sync that keeps marks: it settles every card that differs.
    if !st.backed_up || st.marks.is_empty() {
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
    // A winner not copied everywhere: its folder keeps the old mark, so it still counts as
    // changed next time.
    let mut unsent: BTreeSet<(String, String)> = BTreeSet::new();
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
    // 2. Every folder gets every card. The winner: of the copies changed since the last
    // sync (all when none did), the highest (freshness, file time, folder key).
    let names: BTreeSet<String> = folders.iter().flat_map(|f| f.cards.keys().cloned()).collect();
    for name in names {
        let changed = |f: &Folder| f.cards.get(&name).is_some_and(|c| st.marks.get(&f.key).and_then(|m| m.get(&name)) != Some(&mark(c)));
        let any_changed = folders.iter().any(changed);
        let rank = |f: &Folder| (freshness(&f.cards[&name]), f.mtimes.get(&name).copied().unwrap_or(0), f.key.clone());
        let Some(bi) = (0..folders.len())
            .filter(|&i| folders[i].cards.contains_key(&name) && (!any_changed || changed(&folders[i])))
            .max_by(|&a, &b| {
                let (x, y) = (rank(&folders[a]), rank(&folders[b]));
                x.0.total_cmp(&y.0).then(x.1.cmp(&y.1)).then(x.2.cmp(&y.2))
            })
        else {
            continue;
        };
        let best = folders[bi].cards[&name].clone();
        let (best_mark, winner) = (mark(&best), folders[bi].key.clone());
        for (j, f) in folders.iter_mut().enumerate().filter(|(j, f)| *j != bi && f.readable) {
            let new = match f.cards.get(&name) {
                // An unreadable card of that name stays as it is.
                None if f.present.contains(&name) => continue,
                None => fresh_copy(&best, templates[j].as_ref()),
                Some(cur) if mark(cur) != best_mark => merged(&best, cur),
                Some(_) => continue,
            };
            if let Err(e) = write_card(f, &name, &new) {
                errs.push(e);
                unsent.insert((winner.clone(), name.clone()));
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
        // What it holds now: a write that failed is not a change next time.
        let old = st.marks.remove(&f.key).unwrap_or_default();
        let marks = f.cards.iter().filter_map(|(n, c)| if unsent.contains(&(f.key.clone(), n.clone())) { old.get(n).map(|m| (n.clone(), m.clone())) } else { Some((n.clone(), mark(c))) });
        st.marks.insert(f.key.clone(), marks.collect());
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

#[cfg(test)]
#[path = "sessions_tests.rs"]
mod tests;
