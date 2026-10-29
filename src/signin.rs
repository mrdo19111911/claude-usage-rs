// Adding an account by signing in in the browser (OAuth).

use crate::*;

impl App {
    // ----- Adding a Claude Code account -----
    // Sign-in in the browser (login.rs): the dashboard keeps the new account's tokens as
    // its own and leaves the login Claude Code uses now as it is.
    pub(crate) fn start_login(&self, email: Option<&str>) -> Result<String, String> {
        if self.demo {
            return Err("Not in demo mode.".into());
        }
        let p = login::start(now());
        let mut url = login::authorize_url(&p, CLIENT_ID, self.port);
        if let Some(e) = email.filter(|e| valid_email(e)) {
            url.push_str(&format!("&login_hint={}", login::encode(e)));
        }
        *self.login.lock().unwrap() = Some(p);
        open_browser(&url);
        Ok(url)
    }

    // The browser's return from the sign-in page; the reply is the page it shows.
    pub(crate) fn finish_login(&self, query: &str) -> Result<String, String> {
        let p = {
            let mut l = self.login.lock().unwrap();
            let fresh = l.as_ref().is_some_and(|p| now() - p.started < login::PENDING_MS);
            let ours = l.as_ref().is_some_and(|p| login::param(query, "state").as_deref() == Some(p.state.as_str()));
            if !fresh || !ours {
                return Err("This sign-in is not one the dashboard started, or it is older than 15 minutes. Click + Add account again.".into());
            }
            l.take().unwrap()
        };
        if let Some(e) = login::param(query, "error") {
            let why = login::param(query, "error_description").unwrap_or(e);
            return Err(format!("Sign-in was not completed: {why}"));
        }
        let code = login::param(query, "code").ok_or("The sign-in page sent no code back.")?;
        let token: Value = self.http
            .post(TOKEN_URLS[0])
            .send_json(login::exchange_body(&p, &code, CLIENT_ID, self.port))
            .map_err(|e| match e {
                ureq::Error::Status(c, r) => format!("Anthropic refused the sign-in ({c}): {}", r.into_string().unwrap_or_default().chars().take(200).collect::<String>()),
                e => e.to_string(),
            })?
            .into_json()
            .map_err(|e| e.to_string())?;
        let mut c = json!({"accessToken": token["access_token"]});
        let profile = self.get_profile_json(&c).map_err(|e| e.text())?;
        c = login::creds(&token, &profile, now());
        let uuid = profile["account"]["uuid"].as_str().ok_or("Anthropic's profile names no account.")?.to_string();
        let email = profile["account"]["email"].as_str().unwrap_or("").to_string();
        let mut st = self.st.lock().unwrap();
        let active = st.active_id.clone();
        let msg = if let Some(a) = st.accounts.iter_mut().find(|a| a.account_uuid.as_deref() == Some(uuid.as_str())) {
            if active.as_deref() == Some(a.id.as_str()) {
                // Its tokens are Claude Code's; keeping a second set would make it look new.
                format!("{email} is the account Claude Code uses now; it is here already.")
            } else {
                a.creds = c;
                a.email = email.clone();
                a.plan = plan_label(&a.creds);
                a.oauth_account = login::oauth_account(&profile);
                a.error = None;
                a.enabled = true;
                format!("Signed in again to {email}")
            }
        } else {
            let id = self.new_id();
            st.accounts.push(Account {
                id,
                account_uuid: Some(uuid),
                email: email.clone(),
                plan: plan_label(&c),
                enabled: true,
                creds: c,
                oauth_account: login::oauth_account(&profile),
                usage: None,
                history: vec![],
                error: None,
                raw: Value::Null,
                slow_until: 0,
                slow_strikes: 0,
                next_check: 0,
            });
            format!("Added {email} (signed in in the browser)")
        };
        if !msg.ends_with("it is here already.") {
            self.log(&mut st, msg.clone());
        }
        self.save(&st);
        Ok(msg)
    }
}

// What the browser shows after signing in.
pub(crate) fn login_page(ok: bool, msg: &str, port: u16) -> String {
    let esc = msg.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
    let (title, color) = if ok { ("Account added", "#3fb950") } else { ("Sign-in not completed", "#f85149") };
    format!(
        r#"<!doctype html><meta charset="utf-8"><title>{title}</title>
<body style="font:16px system-ui;background:#0d1117;color:#e6edf3;display:grid;place-items:center;height:100vh;margin:0">
<div style="text-align:center;max-width:520px;padding:16px"><h1 style="color:{color};font-size:22px">{title}</h1><p>{esc}</p>
<p><a style="color:#58a6ff" href="http://127.0.0.1:{port}/">Back to the dashboard</a></p></div>"#
    )
}
