// Switching Claude Desktop between saved accounts.

use super::{copy_cookies, is_sign_in_key, read_config, sessions, signed_in_of, valid_account_id, write_config, App, Store};
use serde_json::{Map, Value};
use std::fs;
use std::path::Path;

/// Removes the app's sign-in from `config` and puts `auth`'s in (none: signed out).
fn put_sign_in(config: &mut Map<String, Value>, auth: Option<&Map<String, Value>>) {
    config.retain(|k, _| !is_sign_in_key(k));
    if let Some(a) = auth {
        for (k, v) in a.iter().filter(|(k, _)| is_sign_in_key(k)) {
            config.insert(k.clone(), v.clone());
        }
    }
}

/// What a switch did: the account whose sign-in was saved on the way out, and the
/// Code sessions shared on the way (None: not asked for).
pub struct Switched {
    #[allow(dead_code)] // only the tests read it
    pub saved: Option<String>,
    pub shared: Option<Result<sessions::Counts, String>>,
}

/// The switch itself. `target`: the account to sign in to, or None to leave the app
/// signed out (its sign-in screen then adds another account). `share`: where the
/// session sync keeps its state, None when the setting is off (docs/specs/desktop-sessions.md).
/// A failure after the backup puts the app's sign-in back as it was; the app is
/// started again whatever happened, a failed sync included.
pub fn switch(dir: &Path, store: &Store, target: Option<&str>, app: &dyn App, stamp: i64, share: Option<&Path>) -> Result<Switched, String> {
    let target_auth = match target {
        Some(t) => {
            if !valid_account_id(t) {
                return Err("not an account id".into());
            }
            let text = fs::read_to_string(store.profile(t).join("auth.json")).map_err(|_| "no saved sign-in for this account".to_string())?;
            match serde_json::from_str(&text) {
                Ok(Value::Object(m)) => Some(m),
                _ => return Err("saved sign-in is unreadable".into()),
            }
        }
        None => None,
    };
    let stopped = app.stop();
    let res = stopped.and_then(|_| {
        let backup = store.backup(dir, stamp)?;
        let swap = || -> Result<Option<String>, String> {
            // Read only now: the app may write its sign-in while it quits.
            let mut config = read_config(dir)?;
            let current = signed_in_of(&config);
            if let Some(c) = &current {
                store.save(dir, &config, c)?;
            }
            put_sign_in(&mut config, target_auth.as_ref());
            write_config(dir, &config)?;
            match target {
                Some(t) => copy_cookies(&store.profile(t), &dir.join("Network"))?,
                None => copy_cookies(&store.root.join("no-cookies"), &dir.join("Network"))?,
            }
            Ok(current)
        };
        swap().map_err(|e| {
            let restored = fs::copy(backup.join("config.json"), dir.join("config.json")).map_err(|x| x.to_string()).and_then(|_| copy_cookies(&backup, &dir.join("Network")));
            match restored {
                Ok(()) => format!("{e} (sign-in put back as it was)"),
                Err(x) => format!("{e}; putting the sign-in back failed too: {x}, backup in {}", backup.display()),
            }
        })
    });
    // Code sessions are shared while the app is closed; a failure is only reported.
    let shared = match (&res, share) {
        (Ok(_), Some(state_dir)) => Some(sessions::sync(dir, state_dir, stamp)),
        _ => None,
    };
    let started = app.start();
    let saved = res?;
    started?;
    Ok(Switched { saved, shared })
}
