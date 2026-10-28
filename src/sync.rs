// Noticing a new Claude Code login or Claude Desktop sign-in, and Claude Desktop's account names.

use crate::*;

impl App {
    // Work out which stored account Claude Code uses now. A login we do not know by its
    // refresh token is looked up at Anthropic: a new token of a stored account (Claude
    // Code refreshed it, or you signed in to it again) or a new account. Takes the lock.
    pub(crate) fn sync_active(&self) {
        if self.demo {
            return;
        }
        let live = self.read_live();
        {
            let mut st = self.st.lock().unwrap();
            match &live {
                None => {
                    st.active_id = None;
                    return;
                }
                Some(l) if self.note_live(&mut st, l) => return,
                _ => {}
            }
        }
        let live = live.unwrap();
        // Outside the lock: a network call.
        let who = self.get_profile(&live);
        let oa = self.read_oauth_account();
        let mut st = self.st.lock().unwrap();
        if self.note_live(&mut st, &live) {
            return;
        }
        let (uuid, email) = match who {
            Ok(w) => w,
            Err(e) => {
                // Unknown whose it is: no account counts as active until it is known.
                st.active_id = None;
                println!("Could not identify Claude Code's login: {}", e.text());
                return;
            }
        };
        if let Some(a) = st.accounts.iter_mut().find(|a| a.account_uuid.as_deref() == Some(uuid.as_str())) {
            a.creds = live;
            a.email = email;
            a.error = None;
            keep_oauth_account(a, oa);
            st.active_id = Some(a.id.clone());
        } else {
            let id = self.new_id();
            let mut a = Account {
                id: id.clone(),
                account_uuid: Some(uuid),
                email: email.clone(),
                plan: plan_label(&live),
                enabled: true,
                creds: live,
                oauth_account: Value::Null,
                usage: None,
                history: vec![],
                error: None,
                raw: Value::Null,
                slow_until: 0,
                slow_strikes: 0,
                next_check: 0,
            };
            keep_oauth_account(&mut a, oa);
            st.accounts.push(a);
            st.active_id = Some(id);
            self.log(&mut st, format!("Added {email} (new Claude Code login)"));
        }
        self.save(&st);
    }

    // Gives the account Claude Desktop is signed in to now an entry, if it has none.
    pub(crate) fn sync_desktop(&self, st: &mut State) {
        let Some(dir) = &self.desktop_dir else { return };
        let Some(uuid) = desktop::signed_in(dir) else { return };
        // Signed in again after a sign-out for it: the wait is over.
        if !self.switching.load(Ordering::SeqCst) {
            self.desktop_expect.lock().unwrap().take();
        }
        if st.desktop.iter().any(|d| d.uuid == uuid) {
            return;
        }
        let label = noted_email(self.data_dir(), &uuid).unwrap_or_else(|| format!("Account {}", &uuid[..8]));
        st.desktop.push(DesktopAccount { uuid, label: label.clone() });
        self.log(st, format!("Claude Desktop: added {label} (signed in now)"));
        self.save(st);
    }
}

// ---------- Claude Desktop names ----------
// Claude Desktop keeps no readable email, but it hands each Code session the account's
// uuid and email (CLAUDE_CODE_ACCOUNT_UUID, CLAUDE_CODE_USER_EMAIL). A SessionStart hook
// runs `claude-usage --note-session`, which saves them as desktop-names/<uuid>.
pub(crate) fn names_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("desktop-names")
}

pub(crate) fn valid_email(s: &str) -> bool {
    s.len() <= 200 && s.contains('@') && !s.chars().any(|c| c.is_control() || c.is_whitespace())
}

pub(crate) fn note_session(data_dir: &Path, uuid: &str, email: &str) -> bool {
    if !desktop::valid_account_id(uuid) || !valid_email(email) {
        return false;
    }
    let file = names_dir(data_dir).join(uuid);
    if fs::read_to_string(&file).is_ok_and(|s| s == email) {
        return true;
    }
    fs::create_dir_all(names_dir(data_dir)).is_ok() && write_private(&file, email).is_ok()
}

pub(crate) fn noted_email(data_dir: &Path, uuid: &str) -> Option<String> {
    if !desktop::valid_account_id(uuid) {
        return None;
    }
    fs::read_to_string(names_dir(data_dir).join(uuid)).ok().filter(|s| valid_email(s))
}
