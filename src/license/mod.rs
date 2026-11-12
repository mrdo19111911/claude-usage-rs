// Selling SCAS (docs/specs/license.md): the 30-day trial counted per Claude email, a paid
// Polar key, ads after the trial. Pure rules here; the network part comes later.
#![allow(dead_code)] // wired into auto-switch and the dashboard in a later step

use sha2::{Digest, Sha256};

pub(crate) const DAY: i64 = 86_400_000;
const TRIAL_DAYS: i64 = 30;
// A paid key works this long without reaching Polar.
const OFFLINE_DAYS: i64 = 7;
// Fixed: the same email gives the same hash on every computer.
const SALT: &str = "scas-trial-v1:";

// What the trial server stores for an email: never the email itself.
pub(crate) fn email_hash(email: &str) -> String {
    let h = Sha256::digest(format!("{SALT}{}", email.trim().to_lowercase()).as_bytes());
    h.iter().map(|b| format!("{b:02x}")).collect()
}

// The trial's start from the server's answer per email (None: not seen before): the
// earliest one, so adding a new email never restarts it; all new: now.
pub(crate) fn trial_start(seen: &[Option<i64>], now: i64) -> i64 {
    seen.iter().flatten().copied().min().unwrap_or(now).min(now)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum KeyState {
    None,
    Active,
    Revoked,
}

// What license.json keeps.
#[derive(Clone, Debug)]
pub(crate) struct Saved {
    pub(crate) trial_start: Option<i64>,
    pub(crate) key: KeyState,
    // Last time Polar answered for the key.
    pub(crate) checked_at: i64,
    // The user chose to watch ads after the trial.
    pub(crate) ads: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Status {
    // Never pays: 1-2 accounts, or an account with "gosmartlog" in it.
    Free { why: &'static str },
    Trial { days_left: i64 },
    Licensed,
    Ads,
    Expired { why: &'static str },
}

pub(crate) fn status(s: &Saved, now: i64) -> Status {
    match s.key {
        KeyState::Revoked => return Status::Expired { why: "revoked" },
        KeyState::Active if now - s.checked_at <= OFFLINE_DAYS * DAY => return Status::Licensed,
        KeyState::Active => return Status::Expired { why: "not checked for 7 days" },
        KeyState::None => {}
    }
    let left = TRIAL_DAYS - (now - s.trial_start.unwrap_or(now)).max(0) / DAY;
    if left > 0 {
        Status::Trial { days_left: left }
    } else if s.ads {
        Status::Ads
    } else {
        Status::Expired { why: "trial over" }
    }
}

// Accounts free without a key (docs/specs/license.md, "Free").
const FREE_ACCOUNTS: usize = 2;
const FREE_MARK: &str = "gosmartlog";

// The status for the Claude accounts in SCAS (`emails`): free for 1-2 different accounts or
// any "gosmartlog" one, else the trial / key rules.
pub(crate) fn status_for(s: &Saved, emails: &[&str], now: i64) -> Status {
    let mut l: Vec<String> = emails.iter().map(|e| e.trim().to_lowercase()).collect();
    l.sort();
    l.dedup();
    if l.iter().any(|e| e.contains(FREE_MARK)) {
        Status::Free { why: "gosmartlog" }
    } else if l.len() <= FREE_ACCOUNTS {
        Status::Free { why: "1-2 accounts" }
    } else {
        status(s, now)
    }
}

// Auto-switch runs unless the license has expired; usage and switching by hand always work.
pub(crate) fn allows_auto(s: &Status) -> bool {
    !matches!(s, Status::Expired { .. })
}

#[cfg(test)]
mod tests;
