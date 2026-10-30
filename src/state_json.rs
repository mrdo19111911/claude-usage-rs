// What the page (and the tray widget) gets from /api/state; the demo state.

use crate::*;

impl App {
    pub(crate) fn public_state(&self) -> String {
        let st = self.st.lock().unwrap();
        let t = now();
        let accounts: Vec<Value> = st.accounts.iter().map(|a| json!({
            "id": a.id, "uuid": a.account_uuid, "email": a.email, "plan": a.plan, "enabled": a.enabled, "nextCheck": a.next_check,
            "usage": a.usage, "error": a.error, "raw": a.raw,
            "history": thin_history(&a.history, t),
        })).collect();
        let desktop_now = self.desktop_dir.as_deref().and_then(desktop::signed_in);
        let desktop: Vec<Value> = st.desktop.iter().map(|d| {
            // Usage is per account: the terminal login of the same account shows it.
            let cli = st.accounts.iter().find(|a| a.account_uuid.as_deref() == Some(d.uuid.as_str()));
            json!({
                "uuid": d.uuid, "label": d.label,
                "email": cli.map(|a| a.email.clone()).or_else(|| noted_email(self.data_dir(), &d.uuid)),
                "usage": cli.and_then(|a| a.usage.clone()),
                "hasSignIn": self.desktop_store.has_sign_in(&d.uuid),
                "signedIn": desktop_now.as_deref() == Some(d.uuid.as_str()),
            })
        }).collect();
        json!({
            "desktop": {
                "found": self.desktop_dir.is_some(),
                "dir": self.desktop_dir.as_ref().map(|d| d.display().to_string()),
                "switching": self.switching.load(Ordering::Relaxed),
                "expect": *self.desktop_expect.lock().unwrap(),
                "accounts": desktop,
            },
            "now": t,
            "lastPollAt": self.last_poll.load(Ordering::Relaxed),
            "polling": self.polling.load(Ordering::Relaxed),
            "loggingIn": self.login.lock().unwrap().as_ref().is_some_and(|p| t - p.started < login::PENDING_MS),
            "demo": self.demo,
            "version": update::VERSION,
            "update": *self.update.lock().unwrap(),
            "settings": st.settings,
            "activeId": st.active_id,
            "events": st.events.iter().take(15).collect::<Vec<_>>(),
            "accounts": accounts,
            "loginFile": if self.keychain { "macOS Keychain: Claude Code-credentials".to_string() } else { self.creds_file.display().to_string() },
            "loginFound": self.demo || self.read_live().is_some(),
            "loginEmail": self.read_oauth_account().get("emailAddress").cloned().unwrap_or(Value::Null),
            "claudeJson": self.claude_json.display().to_string(),
            "gemini": gemini::public(&st),
            "geminiWatch": self.gemini_watch.public(),
            "usageLog": self.usage_log.stats(),
        })
        .to_string()
    }
}

pub(crate) fn demo_state(app: &App) -> State {
    let t = now();
    let mk = |email: &str, five: i64, week: i64, week_reset_h: i64, five_reset: Option<i64>, hist: Vec<(i64, i64, i64)>| Account {
        id: app.new_id(),
        account_uuid: None,
        email: email.into(),
        plan: "Max 20x".into(),
        enabled: true,
        creds: json!({}),
        oauth_account: Value::Null,
        usage: Some(Usage { five: Win { u: five, resets_at: five_reset }, week: Win { u: week, resets_at: Some(t + week_reset_h * HOUR) }, fetched_at: t - 12_000, scoped: vec![Scoped { name: "Fable".into(), u: week / 3, resets_at: Some(t + week_reset_h * HOUR) }] }),
        history: hist,
        error: None,
        raw: Value::Null,
        slow_until: 0,
        slow_strikes: 0,
        next_check: 0,
    };
    let start = t - 126 * 60_000;
    let pts = [(0, 0), (75, 2), (79, 3), (82, 3), (90, 5), (99, 7), (108, 7), (118, 7), (125, 9)]
        .iter()
        .map(|&(m, u)| (start + m * 60_000, u, 24))
        .collect();
    let accounts = vec![
        mk("lead@example.com", 9, 24, 59, Some(start + 5 * HOUR), pts),
        mk("dev@example.com", 0, 96, 107, None, vec![]),
        mk("qa@example.com", 0, 100, 18, None, vec![]),
        mk("admin@example.com", 0, 99, 45, None, vec![]),
        mk("steve@example.com", 0, 100, 130, None, vec![]),
        mk("ops@example.com", 0, 99, 140, None, vec![]),
    ];
    let active_id = Some(accounts[0].id.clone());
    State { accounts, active_id, ..Default::default() }
}
