// A saved sign-in renewed or refused (docs/specs/modules.md, "A renewed sign-in is kept at
// once"). Refresh tokens are single-use: the new ones are stored and saved before anything
// else can fail.

use crate::*;

impl App {
    // Refreshes `id`'s saved sign-in and keeps the answer. Never for the live Claude Code
    // login (its refresh token is Claude Code's own). Call it without the state lock.
    pub(crate) fn renew(&self, id: &str, creds: &Value) -> Result<Value, ApiErr> {
        let c = self.refresh(creds)?;
        self.keep_renewed(id, &c);
        Ok(c)
    }

    pub(crate) fn keep_renewed(&self, id: &str, creds: &Value) {
        let mut st = self.st.lock().unwrap();
        let Some(a) = st.accounts.iter_mut().find(|a| a.id == id) else { return };
        a.creds = creds.clone();
        self.save(&st);
    }

    // A switch found `id`'s sign-in refused, or another account's: its card says `why`, and
    // auto-switch passes it by until it is signed in again or a check clears it.
    pub(crate) fn refuse(&self, id: &str, why: &str) {
        let mut st = self.st.lock().unwrap();
        let Some(a) = st.accounts.iter_mut().find(|a| a.id == id) else { return };
        a.error = Some(why.to_string());
        a.waiting = false;
        a.next_check = now() + RETRY_MS;
        self.save(&st);
    }
}

// The request never left this machine, so its refresh token cannot have been spent: only
// then is the second token address tried.
pub(crate) fn not_sent(kind: ureq::ErrorKind) -> bool {
    matches!(kind, ureq::ErrorKind::Dns | ureq::ErrorKind::ConnectionFailed)
}
