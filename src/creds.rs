// Claude Code's own login: its credentials file (or macOS Keychain) and ~/.claude.json.

use crate::*;

pub(crate) const KEYCHAIN_SERVICE: &str = "Claude Code-credentials";

impl App {
    // ----- Claude Code's live login -----
    pub(crate) fn read_live(&self) -> Option<Value> {
        let raw = if self.keychain {
            let o = Command::new("security")
                .args(["find-generic-password", "-s", KEYCHAIN_SERVICE, "-w"])
                .output()
                .ok()?;
            String::from_utf8(o.stdout).ok()?
        } else {
            fs::read_to_string(&self.creds_file).ok()?
        };
        let v: Value = serde_json::from_str(raw.trim()).ok()?;
        v.get("claudeAiOauth").filter(|c| c.is_object()).cloned()
    }

    pub(crate) fn write_live(&self, oauth: &Value) -> Result<(), String> {
        if self.keychain {
            let mut obj = Command::new("security")
                .args(["find-generic-password", "-s", KEYCHAIN_SERVICE, "-w"])
                .output()
                .ok()
                .and_then(|o| serde_json::from_slice::<Value>(&o.stdout).ok())
                .filter(|v| v.is_object())
                .unwrap_or(json!({}));
            obj["claudeAiOauth"] = oauth.clone();
            let user = std::env::var("USER").unwrap_or_default();
            let ok = Command::new("security")
                .args(["add-generic-password", "-U", "-s", KEYCHAIN_SERVICE, "-a", &user, "-w", &obj.to_string()])
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            return if ok { Ok(()) } else { Err("could not write Keychain".into()) };
        }
        let mut obj: Value = fs::read_to_string(&self.creds_file)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .filter(|v: &Value| v.is_object())
            .unwrap_or(json!({}));
        obj["claudeAiOauth"] = oauth.clone();
        if let Some(dir) = self.creds_file.parent() {
            let _ = fs::create_dir_all(dir);
        }
        write_private(&self.creds_file, &obj.to_string()).map_err(|e| e.to_string())
    }

    pub(crate) fn read_oauth_account(&self) -> Value {
        fs::read_to_string(&self.claude_json)
            .ok()
            .and_then(|s| serde_json::from_str::<Value>(&s).ok())
            .and_then(|v| v.get("oauthAccount").cloned())
            .unwrap_or(Value::Null)
    }

    // Claude Code shows the account named here. Written only when it is known to be
    // the account's own (keep_oauth_account), never another's.
    pub(crate) fn write_oauth_account(&self, acc: &Account) {
        let oa = &acc.oauth_account;
        if oa.is_null() || acc.account_uuid.is_none() || oa.get("accountUuid").and_then(Value::as_str) != acc.account_uuid.as_deref() {
            return;
        }
        // Under Claude Code's own lock on the file, re-read just before: only this key changes.
        let Some(_held) = lock::config(&self.claude_json) else {
            println!("~/.claude.json is locked by Claude Code; its shown account was left as it is");
            return;
        };
        let Some(mut obj) = fs::read_to_string(&self.claude_json)
            .ok()
            .and_then(|s| serde_json::from_str::<Value>(&s).ok())
            .filter(|v| v.is_object())
        else {
            return;
        };
        obj["oauthAccount"] = oa.clone();
        let _ = write_private(&self.claude_json, &serde_json::to_string_pretty(&obj).unwrap());
    }

    // Takes the stored account whose refresh token Claude Code holds now as the active
    // one, with the live tokens (Claude Code may have refreshed them). False when the
    // live login is none we know by its refresh token.
    pub(crate) fn note_live(&self, st: &mut State, live: &Value) -> bool {
        let oa = self.read_oauth_account();
        let rt = live.get("refreshToken").filter(|v| v.is_string());
        let Some(a) = st.accounts.iter_mut().find(|a| rt.is_some() && a.creds.get("refreshToken") == rt) else { return false };
        if a.creds.get("accessToken") != live.get("accessToken") {
            a.creds = live.clone();
            a.error = None;
        }
        keep_oauth_account(a, oa);
        st.active_id = Some(a.id.clone());
        true
    }
}

// An access token this close to its end is renewed first (Claude Code: 5 min ahead).
pub(crate) fn expires_soon(c: &Value) -> bool {
    c.get("expiresAt").and_then(Value::as_i64).unwrap_or(0) - 60_000 < now()
}

// Keeps ~/.claude.json's oauthAccount for `a` only when it names a's own account.
pub(crate) fn keep_oauth_account(a: &mut Account, oa: Value) {
    if a.account_uuid.is_some() && oa.get("accountUuid").and_then(Value::as_str) == a.account_uuid.as_deref() {
        a.oauth_account = oa;
    }
}

pub(crate) fn plan_label(c: &Value) -> String {
    let tier = c.get("rateLimitTier").and_then(Value::as_str).unwrap_or("");
    if let Some(i) = tier.find("max_") {
        let n: String = tier[i + 4..].chars().take_while(|c| c.is_ascii_digit()).collect();
        if !n.is_empty() {
            return format!("Max {n}x");
        }
    }
    let sub = c.get("subscriptionType").and_then(Value::as_str).unwrap_or("");
    let mut ch = sub.chars();
    match ch.next() {
        Some(f) => f.to_uppercase().collect::<String>() + ch.as_str(),
        None => "Claude".into(),
    }
}
