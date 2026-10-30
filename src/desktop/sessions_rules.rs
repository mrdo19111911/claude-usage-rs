// How one card becomes another folder's (docs/specs/desktop-sessions.md, "What").
// Same rules as bdarbaz/claude-session-sync (MIT): fresh_copy, merged, templates.

use super::sessions::{Card, Folder};
use serde_json::Value;
use std::collections::BTreeMap;

/// Keys that belong to the account holding the card: never copied across.
pub(super) const ACCOUNT_KEYS: [&str; 9] = [
    "remoteMcpServersConfig",
    "enabledMcpTools",
    "toolSurfaceSnapshot",
    "bridgeSessionIds",
    "remoteControlAutoEligible",
    "steeredByRemoteClient",
    "error",
    "errorAt",
    "priorErrorMark",
];

/// Of those, what a new copy takes from the target folder's own cards.
const TEMPLATE_KEYS: [&str; 3] = ["remoteMcpServersConfig", "enabledMcpTools", "toolSurfaceSnapshot"];

fn is_account_key(k: &str) -> bool {
    ACCOUNT_KEYS.contains(&k)
}

/// max(lastActivityAt, lastFocusedAt), 0 when neither.
pub(super) fn freshness(c: &Card) -> f64 {
    ["lastActivityAt", "lastFocusedAt"].iter().filter_map(|k| c.get(*k).and_then(Value::as_f64)).fold(0.0, f64::max)
}

/// A card for a folder that has none: `src` minus the account keys, plus the folder's
/// connector keys from `template`.
pub(super) fn fresh_copy(src: &Card, template: Option<&Card>) -> Card {
    let mut c: Card = src.iter().filter(|(k, _)| !is_account_key(k)).map(|(k, v)| (k.clone(), v.clone())).collect();
    if let Some(t) = template {
        for (k, v) in t {
            c.insert(k.clone(), v.clone());
        }
    }
    c
}

/// `dst` brought up to `src`: src's keys, dst's own account keys.
pub(super) fn merged(src: &Card, dst: &Card) -> Card {
    let mut m: Card = src.iter().filter(|(k, _)| !is_account_key(k)).map(|(k, v)| (k.clone(), v.clone())).collect();
    for (k, v) in dst.iter().filter(|(k, _)| is_account_key(k)) {
        m.insert(k.clone(), v.clone());
    }
    m
}

/// Each folder's connector keys: from its own freshest card this app did not copy there
/// (one that has remoteMcpServersConfig, as claude-session-sync asks), else from another
/// folder of the same account, else none.
pub(super) fn templates(folders: &[Folder], copied: &BTreeMap<String, Vec<String>>) -> Vec<Option<Card>> {
    let own: Vec<Option<Card>> = folders
        .iter()
        .map(|f| {
            let ours = copied.get(&f.key);
            f.cards
                .iter()
                .filter(|(n, c)| !ours.is_some_and(|o| o.contains(n)) && c.get("remoteMcpServersConfig").is_some_and(|v| !v.is_null()))
                .map(|(_, c)| c)
                .fold(None, |best: Option<&Card>, c| match best {
                    Some(b) if freshness(b) >= freshness(c) => Some(b),
                    _ => Some(c),
                })
                .map(|c| TEMPLATE_KEYS.iter().filter_map(|k| c.get(*k).map(|v| (k.to_string(), v.clone()))).collect())
        })
        .collect();
    folders
        .iter()
        .zip(&own)
        .map(|(f, t)| {
            t.clone().or_else(|| folders.iter().zip(&own).find(|(g, t)| g.account == f.account && t.is_some()).and_then(|(_, t)| t.clone()))
        })
        .collect()
}
