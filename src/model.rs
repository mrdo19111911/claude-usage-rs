// Saved state: settings, accounts, their usage and history, events.

use crate::*;

pub(crate) const HOUR: i64 = 3_600_000;

// ---------- state ----------
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Settings {
    pub(crate) threshold: i64,
    pub(crate) auto_switch: bool,
    pub(crate) poll_seconds: u64,
    // Install a newer release as soon as a check finds it (absent in older saved state: on).
    #[serde(default = "yes")]
    pub(crate) auto_update: bool,
}

fn yes() -> bool {
    true
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Win {
    pub(crate) u: i64,
    pub(crate) resets_at: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Usage {
    pub(crate) five: Win,
    pub(crate) week: Win,
    pub(crate) fetched_at: i64,
    // Weekly limits of one model each (Anthropic's "weekly_scoped"), e.g. Fable.
    #[serde(default)]
    pub(crate) scoped: Vec<Scoped>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Scoped {
    pub(crate) name: String,
    pub(crate) u: i64,
    pub(crate) resets_at: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Account {
    pub(crate) id: String,
    // The account the token belongs to, as Anthropic's profile endpoint says; None
    // until checked (states written by older versions).
    #[serde(default)]
    pub(crate) account_uuid: Option<String>,
    pub(crate) email: String,
    pub(crate) plan: String,
    pub(crate) enabled: bool,
    pub(crate) creds: Value,
    #[serde(default)]
    pub(crate) oauth_account: Value,
    pub(crate) usage: Option<Usage>,
    #[serde(default)]
    pub(crate) history: Vec<(i64, i64, i64)>,
    pub(crate) error: Option<String>,
    #[serde(default)]
    pub(crate) raw: Value,
    // After a 429: no usage request for this login before `slow_until` (ms), and how
    // many 429s in a row led there.
    #[serde(default)]
    pub(crate) slow_until: i64,
    #[serde(default)]
    pub(crate) slow_strikes: u32,
    // When to ask for this account's usage next (ms): see next_check_at.
    #[serde(default)]
    pub(crate) next_check: i64,
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct Event {
    pub(crate) t: i64,
    pub(crate) msg: String,
}

// An account Claude Desktop has been signed in to. Its sign-in is saved by desktop.rs
// under `uuid`; its usage is the terminal account's with the same uuid, if any.
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DesktopAccount {
    pub(crate) uuid: String,
    pub(crate) label: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct State {
    pub(crate) settings: Settings,
    pub(crate) accounts: Vec<Account>,
    pub(crate) active_id: Option<String>,
    #[serde(default)]
    pub(crate) events: Vec<Event>,
    #[serde(default)]
    pub(crate) desktop: Vec<DesktopAccount>,
}

impl Default for State {
    fn default() -> Self {
        State {
            settings: Settings { threshold: 95, auto_switch: true, poll_seconds: 120, auto_update: true },
            accounts: vec![],
            active_id: None,
            events: vec![],
            desktop: vec![],
        }
    }
}
