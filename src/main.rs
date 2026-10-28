// Claude usage dashboard: theo dõi % usage 5 giờ / tuần của nhiều tài khoản Claude
// và tự chuyển login của Claude Code sang tài khoản còn dư khi chạm ngưỡng.
// Chạy file là xong: tự mở trình duyệt, tự nhận tài khoản mỗi lần bạn /login.
// Claude Desktop (Windows) giữ đăng nhập riêng: desktop.rs đổi nó khi bạn bấm chuyển.

mod desktop;
mod lock;
mod login;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";
const TOKEN_URLS: [&str; 2] = [
    "https://platform.claude.com/v1/oauth/token",
    "https://console.anthropic.com/v1/oauth/token",
];
const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";
// Whose a token is: ~/.claude.json's oauthAccount can name another account than the
// token in .credentials.json (seen on 2026-10-28), so identity comes from here.
const PROFILE_URL: &str = "https://api.anthropic.com/api/oauth/profile";
const KEYCHAIN_SERVICE: &str = "Claude Code-credentials";
const HOUR: i64 = 3_600_000;
const INDEX: &str = include_str!("index.html");

fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64
}

// ---------- state ----------
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Settings {
    threshold: i64,
    auto_switch: bool,
    poll_seconds: u64,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Win {
    u: i64,
    resets_at: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Usage {
    five: Win,
    week: Win,
    fetched_at: i64,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Account {
    id: String,
    // The account the token belongs to, as Anthropic's profile endpoint says; None
    // until checked (states written by older versions).
    #[serde(default)]
    account_uuid: Option<String>,
    email: String,
    plan: String,
    enabled: bool,
    creds: Value,
    #[serde(default)]
    oauth_account: Value,
    usage: Option<Usage>,
    #[serde(default)]
    history: Vec<(i64, i64, i64)>,
    error: Option<String>,
    #[serde(default)]
    raw: Value,
    // After a 429: no usage request for this login before `slow_until` (ms), and how
    // many 429s in a row led there.
    #[serde(default)]
    slow_until: i64,
    #[serde(default)]
    slow_strikes: u32,
    // When to ask for this account's usage next (ms): see next_check_at.
    #[serde(default)]
    next_check: i64,
}

#[derive(Serialize, Deserialize, Clone)]
struct Event {
    t: i64,
    msg: String,
}

// An account Claude Desktop has been signed in to. Its sign-in is saved by desktop.rs
// under `uuid`; its usage is the terminal account's with the same uuid, if any.
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct DesktopAccount {
    uuid: String,
    label: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct State {
    settings: Settings,
    accounts: Vec<Account>,
    active_id: Option<String>,
    #[serde(default)]
    events: Vec<Event>,
    #[serde(default)]
    desktop: Vec<DesktopAccount>,
}

impl Default for State {
    fn default() -> Self {
        State {
            settings: Settings { threshold: 95, auto_switch: true, poll_seconds: 120 },
            accounts: vec![],
            active_id: None,
            events: vec![],
            desktop: vec![],
        }
    }
}

struct App {
    st: Mutex<State>,
    demo: bool,
    data_file: PathBuf,
    creds_file: PathBuf,
    claude_json: PathBuf,
    keychain: bool,
    http: ureq::Agent,
    polling: AtomicBool,
    last_poll: AtomicI64,
    seq: AtomicU64,
    port: u16,
    desktop_dir: Option<PathBuf>,
    desktop_store: desktop::Store,
    switching: AtomicBool,
    login: Mutex<Option<login::Pending>>,
    // The email Claude Desktop is to be signed in to next, after a sign-out for it.
    desktop_expect: Mutex<Option<String>>,
}

impl App {
    fn save(&self, st: &State) {
        if self.demo {
            return;
        }
        if let Some(dir) = self.data_file.parent() {
            let _ = fs::create_dir_all(dir);
        }
        let _ = write_private(&self.data_file, &serde_json::to_string_pretty(st).unwrap());
    }

    fn log(&self, st: &mut State, msg: String) {
        println!("{msg}");
        st.events.insert(0, Event { t: now(), msg });
        st.events.truncate(50);
    }

    fn new_id(&self) -> String {
        format!("{:x}{:x}", now(), self.seq.fetch_add(1, Ordering::Relaxed))
    }

    // ----- Claude Code's live login -----
    fn read_live(&self) -> Option<Value> {
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

    fn write_live(&self, oauth: &Value) -> Result<(), String> {
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

    fn read_oauth_account(&self) -> Value {
        fs::read_to_string(&self.claude_json)
            .ok()
            .and_then(|s| serde_json::from_str::<Value>(&s).ok())
            .and_then(|v| v.get("oauthAccount").cloned())
            .unwrap_or(Value::Null)
    }

    // Claude Code shows the account named here. Written only when it is known to be
    // the account's own (keep_oauth_account), never another's.
    fn write_oauth_account(&self, acc: &Account) {
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
    fn note_live(&self, st: &mut State, live: &Value) -> bool {
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

    // Work out which stored account Claude Code uses now. A login we do not know by its
    // refresh token is looked up at Anthropic: a new token of a stored account (Claude
    // Code refreshed it, or you signed in to it again) or a new account. Takes the lock.
    fn sync_active(&self) {
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

    // ----- Claude Desktop -----
    fn data_dir(&self) -> &Path {
        self.data_file.parent().unwrap_or(Path::new("."))
    }

    // Gives the account Claude Desktop is signed in to now an entry, if it has none.
    fn sync_desktop(&self, st: &mut State) {
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

    // Runs a desktop switch in the background: it closes and restarts Claude Desktop,
    // which takes up to a minute. `target` None leaves it signed out, to add an account.
    fn start_desktop_switch(self: &Arc<Self>, target: Option<String>) -> Result<(), String> {
        let dir = self.desktop_dir.clone().ok_or("Claude Desktop's data folder was not found")?;
        if let Some(t) = &target {
            if !self.desktop_store.has_sign_in(t) {
                return Err("No sign-in is saved for this account yet. Sign in to it in Claude Desktop once (Add account).".into());
            }
            if desktop::signed_in(&dir).as_deref() == Some(t.as_str()) {
                return Err("Claude Desktop is signed in to this account already.".into());
            }
        }
        #[cfg(windows)]
        let real = desktop::RealApp { data_dir: dir.clone() };
        #[cfg(windows)]
        if real.inside_claude() {
            return Err("This dashboard was started from inside Claude, so closing Claude would stop it half-way. Start it from Explorer or the Start menu.".into());
        }
        #[cfg(not(windows))]
        return Err("Switching Claude Desktop works on Windows only.".into());
        #[cfg(windows)]
        {
            if self.switching.swap(true, Ordering::SeqCst) {
                return Err("A switch is running already.".into());
            }
            let app = self.clone();
            thread::spawn(move || {
                let res = desktop::switch(&dir, &app.desktop_store, target.as_deref(), &real, now());
                let mut st = app.st.lock().unwrap();
                let name = |st: &State, u: &str| st.desktop.iter().find(|d| d.uuid == u).map(|d| d.label.clone()).unwrap_or_else(|| u[..8].to_string());
                let msg = match (&res, &target) {
                    (Ok(_), Some(t)) => format!("Claude Desktop: switched to {}", name(&st, t)),
                    (Ok(_), None) => "Claude Desktop: signed out here; sign in to the account to add".to_string(),
                    (Err(e), _) => format!("Claude Desktop switch failed: {e}"),
                };
                app.log(&mut st, msg);
                app.save(&st);
                app.switching.store(false, Ordering::SeqCst);
            });
            Ok(())
        }
    }

    // ----- Adding a Claude Code account -----
    // Sign-in in the browser (login.rs): the dashboard keeps the new account's tokens as
    // its own and leaves the login Claude Code uses now as it is.
    fn start_login(&self, email: Option<&str>) -> Result<String, String> {
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
    fn finish_login(&self, query: &str) -> Result<String, String> {
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

    // ----- Anthropic API -----
    fn refresh(&self, creds: &Value) -> Result<Value, ApiErr> {
        let rt = creds.get("refreshToken").and_then(Value::as_str).ok_or(ApiErr::Auth("no refresh token — /login this account again".into()))?;
        let mut last = String::from("refresh failed");
        for url in TOKEN_URLS {
            let r = self.http.post(url).send_json(json!({
                "grant_type": "refresh_token", "refresh_token": rt, "client_id": CLIENT_ID
            }));
            match r {
                Ok(resp) => {
                    let j: Value = resp.into_json().map_err(|e| ApiErr::Other(e.to_string()))?;
                    let mut c = creds.clone();
                    c["accessToken"] = j["access_token"].clone();
                    if j["refresh_token"].is_string() {
                        c["refreshToken"] = j["refresh_token"].clone();
                    }
                    c["expiresAt"] = json!(now() + j["expires_in"].as_i64().unwrap_or(3600) * 1000);
                    return Ok(c);
                }
                Err(ureq::Error::Status(404, _)) => continue,
                Err(ureq::Error::Status(429, resp)) => return Err(ApiErr::Slow(retry_after_ms(resp.header("Retry-After")))),
                Err(ureq::Error::Status(code, resp)) => {
                    let j: Value = resp.into_json().unwrap_or(Value::Null);
                    let why = j["error_description"].as_str().or(j["error"].as_str()).unwrap_or("");
                    let msg = format!("refresh failed ({code}): {why}");
                    // Only a refused grant needs a new sign-in; a server error passes.
                    return Err(if code < 500 { ApiErr::Auth(format!("{msg} — /login this account again")) } else { ApiErr::Other(msg) });
                }
                Err(e) => last = e.to_string(),
            }
        }
        Err(ApiErr::Other(last))
    }

    fn get_usage(&self, creds: &Value) -> Result<Value, ureq::Error> {
        let tok = creds.get("accessToken").and_then(Value::as_str).unwrap_or("");
        self.http
            .get(USAGE_URL)
            .set("Authorization", &format!("Bearer {tok}"))
            .set("anthropic-beta", "oauth-2025-04-20")
            .set("User-Agent", "claude-usage-dashboard/1.0")
            .call()?
            .into_json()
            .map_err(|e| ureq::Error::from(e))
    }

    // The account a token belongs to: (uuid, email).
    fn get_profile(&self, creds: &Value) -> Result<(String, String), ApiErr> {
        let j = self.get_profile_json(creds)?;
        let uuid = j["account"]["uuid"].as_str().ok_or(ApiErr::Other("profile without account".into()))?;
        Ok((uuid.to_string(), j["account"]["email"].as_str().unwrap_or("").to_string()))
    }

    fn get_profile_json(&self, creds: &Value) -> Result<Value, ApiErr> {
        let tok = creds.get("accessToken").and_then(Value::as_str).unwrap_or("");
        self.http
            .get(PROFILE_URL)
            .set("Authorization", &format!("Bearer {tok}"))
            .set("anthropic-beta", "oauth-2025-04-20")
            .set("User-Agent", "claude-usage-dashboard/1.0")
            .call()
            .map_err(|e| api_err(e, "profile"))?
            .into_json()
            .map_err(|e| ApiErr::Other(e.to_string()))
    }

    // Returns the usage JSON and, if tokens were refreshed, the new creds. Refresh
    // tokens are single-use: the live login's belongs to Claude Code alone (a refresh
    // here would leave a running Claude Code holding a dead one), so for it
    // `may_refresh` is false and an expired token waits until Claude Code refreshes it.
    fn fetch_usage(&self, creds: &Value, may_refresh: bool) -> Result<(Value, Option<Value>), ApiErr> {
        let mut c = creds.clone();
        let mut changed = false;
        let expired = expires_soon(&c);
        if expired && !may_refresh {
            return Err(ApiErr::Other("token expired: Claude Code renews it the next time it runs".into()));
        }
        if expired {
            c = self.refresh(&c)?;
            changed = true;
        }
        let j = match self.get_usage(&c).map_err(|e| api_err(e, "usage")) {
            Ok(j) => j,
            Err(ApiErr::Auth(_)) if !changed && !may_refresh => return Err(ApiErr::Other("usage HTTP 401: Claude Code renews the token the next time it runs".into())),
            Err(ApiErr::Auth(_)) if !changed => {
                c = self.refresh(&c)?;
                changed = true;
                self.get_usage(&c).map_err(|e| api_err(e, "usage"))?
            }
            Err(e) => return Err(e),
        };
        Ok((j, changed.then_some(c)))
    }

    // Checks with Anthropic that a saved login works, and is `uuid`'s, before it is
    // handed to Claude Code; renews it first when it has expired (it is parked: only
    // the dashboard holds it). Returns the login to hand over.
    fn verify_login(&self, creds: &Value, uuid: Option<&str>) -> Result<Value, String> {
        let mut c = creds.clone();
        let mut renewed = false;
        if expires_soon(&c) {
            c = self.refresh(&c).map_err(|e| e.text())?;
            renewed = true;
        }
        let who = match self.get_profile(&c) {
            Err(ApiErr::Auth(_)) if !renewed => {
                c = self.refresh(&c).map_err(|e| e.text())?;
                self.get_profile(&c)
            }
            r => r,
        }
        .map_err(|e| format!("could not check this login with Anthropic ({}); not switched", e.text()))?;
        if uuid.is_some_and(|u| u != who.0) {
            return Err("this saved login belongs to another account; not switched".into());
        }
        Ok(c)
    }

    fn poll_account(&self, id: &str, force: bool) {
        let (creds, is_active, known, slow_until, base) = {
            let st = self.st.lock().unwrap();
            match st.accounts.iter().find(|a| a.id == id) {
                Some(a) if force || now() >= a.next_check => {
                    (a.creds.clone(), st.active_id.as_deref() == Some(id), a.account_uuid.is_some(), a.slow_until, st.settings.poll_seconds as i64 * 1000)
                }
                _ => return,
            }
        };
        // Anthropic asked this login to slow down: its last figures stand until then.
        if now() < slow_until {
            return;
        }
        // Also the live login when it is not identified yet (active_id is then None).
        let rt = creds.get("refreshToken").filter(|v| v.is_string());
        let is_live = is_active || (rt.is_some() && self.read_live().is_some_and(|l| l.get("refreshToken") == rt));
        let res = self.fetch_usage(&creds, !is_live);
        // Accounts saved by older versions were named after ~/.claude.json, which can
        // be another account's: ask once whose the token is. Not tied to the usage
        // answer: the usage endpoint can be rate-limited (429) while this one answers.
        let who = if known {
            None
        } else {
            let fresh = res.as_ref().ok().and_then(|(_, c)| c.as_ref());
            self.get_profile(fresh.unwrap_or(&creds)).ok()
        };
        let mut st = self.st.lock().unwrap();
        let Some(a) = st.accounts.iter_mut().find(|a| a.id == id) else { return };
        if let Some((uuid, email)) = who {
            if a.email != email {
                println!("{} is {email}'s login; renamed", a.email);
            }
            a.account_uuid = Some(uuid);
            a.email = email;
            if a.oauth_account.get("accountUuid").and_then(Value::as_str) != a.account_uuid.as_deref() {
                a.oauth_account = Value::Null;
            }
        }
        match res {
            Ok((j, new_creds)) => {
                if let Some(c) = new_creds {
                    // Only a parked account is refreshed here (see fetch_usage).
                    a.creds = c;
                }
                let five = to_win(&j["five_hour"]);
                let week = to_win(&j["seven_day"]);
                let t = now();
                a.history.retain(|p| p.0 > t - 8 * 24 * HOUR);
                a.history.push((t, five.u, week.u));
                let u = Usage { five, week, fetched_at: t };
                a.next_check = next_check_at(&u, base, t);
                a.usage = Some(u);
                a.error = None;
                a.raw = j;
                a.slow_strikes = 0;
            }
            Err(ApiErr::Slow(after)) => {
                // Asking again at once only prolongs it (seen: 429 with Retry-After 0).
                a.slow_strikes += 1;
                let wait = backoff_ms(after, a.slow_strikes);
                a.slow_until = now() + wait;
                a.next_check = a.slow_until;
                a.error = Some(format!("Anthropic asked to slow down (HTTP 429); next check in {} min", (wait + 59_999) / 60_000));
            }
            Err(e) => {
                a.error = Some(e.text());
                a.next_check = now() + base;
            }
        }
    }

    // `force`: every account now (Refresh now); else only those whose next check is due.
    fn poll_all(&self, force: bool) {
        if self.demo || self.polling.swap(true, Ordering::SeqCst) {
            return;
        }
        self.sync_active();
        let ids: Vec<String> = {
            let mut st = self.st.lock().unwrap();
            self.sync_desktop(&mut st);
            let t = now();
            st.accounts.iter().filter(|a| force || t >= a.next_check).map(|a| a.id.clone()).collect()
        };
        if ids.is_empty() {
            self.polling.store(false, Ordering::SeqCst);
            return;
        }
        for id in ids {
            self.poll_account(&id, force);
        }
        let next = self.pick_auto(&mut self.st.lock().unwrap());
        // Outside the lock: activate checks the login with Anthropic first.
        if let Some((id, reason)) = next {
            if let Err(e) = self.activate(&id, &reason) {
                let mut st = self.st.lock().unwrap();
                self.log(&mut st, format!("Auto-switch failed: {e}"));
            }
        }
        self.save(&self.st.lock().unwrap());
        self.last_poll.store(now(), Ordering::Relaxed);
        self.polling.store(false, Ordering::SeqCst);
    }

    // ----- switching -----
    // Makes `id` the account Claude Code uses. Takes the state lock itself; call it
    // without holding it.
    fn activate(&self, id: &str, reason: &str) -> Result<(), String> {
        let (creds, uuid) = {
            let st = self.st.lock().unwrap();
            let a = st.accounts.iter().find(|a| a.id == id).ok_or("not found")?;
            (a.creds.clone(), a.account_uuid.clone())
        };
        // A login Anthropic refuses, or another account's, is never handed over.
        let checked = if self.demo { creds } else { self.verify_login(&creds, uuid.as_deref())? };
        let mut st = self.st.lock().unwrap();
        let acc = {
            let a = st.accounts.iter_mut().find(|a| a.id == id).ok_or("not found")?;
            a.creds = checked;
            a.error = None;
            a.clone()
        };
        if !self.demo {
            // Claude Code's refresh and credential-store locks while its login is read
            // and replaced: it can't renew or save its own in between.
            let home = self.creds_file.parent().unwrap_or(Path::new("."));
            let _held = if self.keychain {
                None
            } else {
                let r = lock::refresh(home).ok_or("Claude Code is renewing its login right now; try again in a few seconds")?;
                let s = lock::storage(home).ok_or("Claude Code is saving its login right now; try again in a few seconds")?;
                Some((r, s))
            };
            // Keep the outgoing account's newest tokens first: once the file is
            // overwritten they are gone, and its stored refresh token may be spent.
            if let Some(live) = self.read_live() {
                if !self.note_live(&mut st, &live) {
                    return Err("Claude Code's current login is not identified yet; try again in a few seconds".into());
                }
            }
            self.write_live(&acc.creds)?;
        }
        if !self.demo {
            self.write_oauth_account(&acc);
        }
        let from = st.active_id.as_ref().and_then(|f| st.accounts.iter().find(|a| &a.id == f)).map(|a| a.email.clone());
        st.active_id = Some(acc.id.clone());
        self.log(&mut st, format!("Switched {} → {} ({reason})", from.unwrap_or("(none)".into()), acc.email));
        self.save(&st);
        Ok(())
    }

    // The account automatic switching moves to, with the reason; None to stay.
    fn pick_auto(&self, st: &mut State) -> Option<(String, String)> {
        if !st.settings.auto_switch {
            return None;
        }
        let th = st.settings.threshold;
        let active = st.active_id.as_ref().and_then(|id| st.accounts.iter().find(|a| &a.id == id)).cloned()?;
        let u = active.usage.as_ref()?;
        if u.five.u < th && u.week.u < th {
            return None;
        }
        let room = |a: &Account| a.usage.as_ref().map(|u| (100 - u.five.u).min(100 - u.week.u)).unwrap_or(0);
        let next = st.accounts.iter()
            .filter(|a| a.enabled && a.id != active.id && a.error.is_none())
            .filter(|a| a.usage.as_ref().is_some_and(|u| u.five.u < th && u.week.u < th))
            .max_by_key(|a| room(a))
            .map(|a| a.id.clone());
        let which = if u.five.u >= th { format!("5-hour {}%", u.five.u) } else { format!("weekly {}%", u.week.u) };
        if next.is_none() {
            let msg = format!("{} hit {which} but no other account has room", active.email);
            if st.events.first().map(|e| &e.msg) != Some(&msg) {
                self.log(st, msg);
            }
        }
        next.map(|id| (id, format!("auto: {which}")))
    }

    fn public_state(&self) -> String {
        let st = self.st.lock().unwrap();
        let t = now();
        let accounts: Vec<Value> = st.accounts.iter().map(|a| json!({
            "id": a.id, "uuid": a.account_uuid, "email": a.email, "plan": a.plan, "enabled": a.enabled, "nextCheck": a.next_check,
            "usage": a.usage, "error": a.error, "raw": a.raw,
            "history": a.history.iter().filter(|p| p.0 > t - 6 * HOUR).collect::<Vec<_>>(),
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
            "settings": st.settings,
            "activeId": st.active_id,
            "events": st.events.iter().take(15).collect::<Vec<_>>(),
            "accounts": accounts,
            "loginFile": if self.keychain { "macOS Keychain: Claude Code-credentials".to_string() } else { self.creds_file.display().to_string() },
            "loginFound": self.demo || self.read_live().is_some(),
            "loginEmail": self.read_oauth_account().get("emailAddress").cloned().unwrap_or(Value::Null),
            "claudeJson": self.claude_json.display().to_string(),
        })
        .to_string()
    }
}

// ---------- Claude Desktop names ----------
// Claude Desktop keeps no readable email, but it hands each Code session the account's
// uuid and email (CLAUDE_CODE_ACCOUNT_UUID, CLAUDE_CODE_USER_EMAIL). A SessionStart hook
// runs `claude-usage --note-session`, which saves them as desktop-names/<uuid>.
fn names_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("desktop-names")
}

fn valid_email(s: &str) -> bool {
    s.len() <= 200 && s.contains('@') && !s.chars().any(|c| c.is_control() || c.is_whitespace())
}

fn note_session(data_dir: &Path, uuid: &str, email: &str) -> bool {
    if !desktop::valid_account_id(uuid) || !valid_email(email) {
        return false;
    }
    let file = names_dir(data_dir).join(uuid);
    if fs::read_to_string(&file).is_ok_and(|s| s == email) {
        return true;
    }
    fs::create_dir_all(names_dir(data_dir)).is_ok() && write_private(&file, email).is_ok()
}

fn noted_email(data_dir: &Path, uuid: &str) -> Option<String> {
    if !desktop::valid_account_id(uuid) {
        return None;
    }
    fs::read_to_string(names_dir(data_dir).join(uuid)).ok().filter(|s| valid_email(s))
}

// ---------- helpers ----------
fn write_private(path: &Path, data: &str) -> std::io::Result<()> {
    let tmp = path.with_extension("cud.tmp");
    {
        let mut o = fs::OpenOptions::new();
        o.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            o.mode(0o600);
        }
        use std::io::Write;
        o.open(&tmp)?.write_all(data.as_bytes())?;
    }
    fs::rename(&tmp, path)
}

// A failed call to Anthropic. Auth: the login was refused. Slow: a 429, with the wait
// Retry-After named, if it did.
enum ApiErr {
    Auth(String),
    Slow(Option<i64>),
    Other(String),
}

impl ApiErr {
    fn text(&self) -> String {
        match self {
            ApiErr::Auth(s) | ApiErr::Other(s) => s.clone(),
            ApiErr::Slow(_) => "Anthropic asked to slow down (HTTP 429)".into(),
        }
    }
}

fn api_err(e: ureq::Error, what: &str) -> ApiErr {
    match e {
        ureq::Error::Status(401, _) => ApiErr::Auth(format!("{what} HTTP 401")),
        ureq::Error::Status(429, resp) => ApiErr::Slow(retry_after_ms(resp.header("Retry-After"))),
        ureq::Error::Status(code, _) => ApiErr::Other(format!("{what} HTTP {code}")),
        e => ApiErr::Other(e.to_string()),
    }
}

// Retry-After in seconds (the form Anthropic sends), as ms.
fn retry_after_ms(v: Option<&str>) -> Option<i64> {
    v?.trim().parse::<f64>().ok().filter(|s| s.is_finite() && *s >= 0.0).map(|s| (s * 1000.0) as i64)
}

const SLOW_DOWN_MS: i64 = 5 * 60_000; // after a 429 that names no wait, as Claude Code waits
const MAX_WAIT_MS: i64 = 60 * 60_000;

// How long to leave a login alone after its `strikes`-th 429 in a row: the wait
// Anthropic named, else 5 min, doubling with each 429 in a row (to 40), at most an hour.
// A Retry-After of 0 names no wait: the next poll would only be refused again.
fn backoff_ms(after: Option<i64>, strikes: u32) -> i64 {
    let ms = match after {
        Some(ms) if ms > 0 => ms,
        _ => SLOW_DOWN_MS << strikes.saturating_sub(1).min(3),
    };
    ms.min(MAX_WAIT_MS)
}

// When to ask for an account's usage next, after figures fetched at `t`.
// A window at 100% cannot change until it resets: no question before then (plus a
// minute, so Anthropic has reset it). Otherwise every `base`, more often near a limit:
// from 70% half of it, from 85% a quarter, from 95% an eighth; never under 15 s.
fn next_check_at(u: &Usage, base_ms: i64, t: i64) -> i64 {
    let full: Vec<i64> = [&u.five, &u.week].iter().filter(|w| w.u >= 100).filter_map(|w| w.resets_at).collect();
    if let Some(r) = full.into_iter().max() {
        if r > t {
            return r + 60_000;
        }
    }
    let used = u.five.u.max(u.week.u);
    let every = match used {
        ..=69 => base_ms,
        70..=84 => base_ms / 2,
        85..=94 => base_ms / 4,
        _ => base_ms / 8,
    };
    t + every.max(15_000)
}

// An access token this close to its end is renewed first (Claude Code: 5 min ahead).
fn expires_soon(c: &Value) -> bool {
    c.get("expiresAt").and_then(Value::as_i64).unwrap_or(0) - 60_000 < now()
}

// Keeps ~/.claude.json's oauthAccount for `a` only when it names a's own account.
fn keep_oauth_account(a: &mut Account, oa: Value) {
    if a.account_uuid.is_some() && oa.get("accountUuid").and_then(Value::as_str) == a.account_uuid.as_deref() {
        a.oauth_account = oa;
    }
}

fn plan_label(c: &Value) -> String {
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

fn to_win(w: &Value) -> Win {
    Win {
        u: w.get("utilization").and_then(Value::as_f64).unwrap_or(0.0).round() as i64,
        resets_at: w.get("resets_at").and_then(Value::as_str).and_then(parse_iso),
    }
}

// "2025-10-28T11:40:00.123+00:00" -> epoch ms
fn parse_iso(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    let n = |a: usize, l: usize| -> Option<i64> { s.get(a..a + l)?.parse().ok() };
    let (y, mo, d, h, mi, se) = (n(0, 4)?, n(5, 2)?, n(8, 2)?, n(11, 2)?, n(14, 2)?, n(17, 2)?);
    let mut i = 19;
    let mut ms = 0;
    if b.get(i) == Some(&b'.') {
        i += 1;
        let start = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        let frac = &s[start..(start + 3).min(i)];
        ms = format!("{frac:0<3}").parse().ok()?;
    }
    let mut off = 0;
    if let Some(&c) = b.get(i) {
        if c == b'+' || c == b'-' {
            off = (n(i + 1, 2)? * 60 + n(i + 4, 2)?) * if c == b'+' { 1 } else { -1 };
        }
    }
    let y2 = if mo <= 2 { y - 1 } else { y };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let doy = (153 * ((mo + 9) % 12) + 2) / 5 + d - 1;
    let days = era * 146_097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719_468;
    Some((days * 86_400 + h * 3600 + mi * 60 + se - off * 60) * 1000 + ms)
}

fn home() -> PathBuf {
    let var = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
    std::env::var_os(var).or_else(|| std::env::var_os("HOME")).map(PathBuf::from).unwrap_or_else(|| ".".into())
}

// What the browser shows after signing in.
fn login_page(ok: bool, msg: &str, port: u16) -> String {
    let esc = msg.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
    let (title, color) = if ok { ("Account added", "#3fb950") } else { ("Sign-in not completed", "#f85149") };
    format!(
        r#"<!doctype html><meta charset="utf-8"><title>{title}</title>
<body style="font:16px system-ui;background:#0d1117;color:#e6edf3;display:grid;place-items:center;height:100vh;margin:0">
<div style="text-align:center;max-width:520px;padding:16px"><h1 style="color:{color};font-size:22px">{title}</h1><p>{esc}</p>
<p><a style="color:#58a6ff" href="http://127.0.0.1:{port}/">Back to the dashboard</a></p></div>"#
    )
}

fn open_browser(url: &str) {
    let _ = if cfg!(windows) {
        // Not `cmd /C start`: cmd would cut the URL at its first '&'.
        Command::new("rundll32").args(["url.dll,FileProtocolHandler", url]).spawn()
    } else if cfg!(target_os = "macos") {
        Command::new("open").arg(url).spawn()
    } else {
        Command::new("xdg-open").arg(url).spawn()
    };
}

fn demo_state(app: &App) -> State {
    let t = now();
    let mk = |email: &str, five: i64, week: i64, week_reset_h: i64, five_reset: Option<i64>, hist: Vec<(i64, i64, i64)>| Account {
        id: app.new_id(),
        account_uuid: None,
        email: email.into(),
        plan: "Max 20x".into(),
        enabled: true,
        creds: json!({}),
        oauth_account: Value::Null,
        usage: Some(Usage { five: Win { u: five, resets_at: five_reset }, week: Win { u: week, resets_at: Some(t + week_reset_h * HOUR) }, fetched_at: t - 12_000 }),
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

// ---------- HTTP ----------
fn handle(app: &Arc<App>, mut rq: tiny_http::Request) {
    let method = rq.method().as_str().to_string();
    let url = rq.url().to_string();
    let path = url.split('?').next().unwrap_or("").to_string();
    let mut body = String::new();
    let _ = rq.as_reader().read_to_string(&mut body);
    let body: Value = serde_json::from_str(&body).unwrap_or(json!({}));
    let from_page = rq.headers().iter().any(|h| h.field.equiv("X-CUD") && h.value.as_str() == "1");
    // Only this machine's own names: a page that points its own domain at 127.0.0.1
    // (DNS rebinding) would otherwise count as the page itself.
    let host_ok = rq.headers().iter().find(|h| h.field.equiv("Host")).is_some_and(|h| {
        let host = h.value.as_str();
        host == format!("127.0.0.1:{}", app.port) || host == format!("localhost:{}", app.port)
    });

    let result: Result<(u16, String, &str), String> = (|| {
        if !host_ok {
            return Ok((403, r#"{"error":"forbidden host"}"#.into(), "application/json"));
        }
        if method == "GET" && (path == "/" || path == "/index.html") {
            return Ok((200, INDEX.to_string(), "text/html; charset=utf-8"));
        }
        if method == "GET" && path == "/api/state" {
            return Ok((200, app.public_state(), "application/json"));
        }
        // The sign-in page sends the browser back here; the state it carries is the check.
        if method == "GET" && path == "/callback" {
            let (ok, msg) = match app.finish_login(&url) {
                Ok(m) => (true, m),
                Err(e) => (false, e),
            };
            if ok {
                let a = app.clone();
                thread::spawn(move || a.poll_all(false));
            }
            return Ok((200, login_page(ok, &msg, app.port), "text/html; charset=utf-8"));
        }
        // Only accept writes from the page itself (blocks cross-site form posts).
        if method != "GET" && !from_page {
            return Ok((403, r#"{"error":"forbidden"}"#.into(), "application/json"));
        }
        match (method.as_str(), path.as_str()) {
            ("POST", "/api/refresh") | ("POST", "/api/import") => {
                app.poll_all(true);
            }
            ("POST", "/api/settings") => {
                let mut st = app.st.lock().unwrap();
                if let Some(v) = body["threshold"].as_f64() {
                    st.settings.threshold = (v as i64).clamp(50, 100);
                }
                if let Some(v) = body["autoSwitch"].as_bool() {
                    st.settings.auto_switch = v;
                }
                if let Some(v) = body["pollSeconds"].as_f64() {
                    st.settings.poll_seconds = (v as u64).max(30);
                }
                app.save(&st);
            }
            ("POST", "/api/login") => {
                let url = app.start_login(body["email"].as_str())?;
                return Ok((200, json!({ "url": url }).to_string(), "application/json"));
            }
            ("POST", "/api/desktop/add") => {
                app.start_desktop_switch(None)?;
                *app.desktop_expect.lock().unwrap() = body["email"].as_str().filter(|e| valid_email(e)).map(String::from);
            }
            _ if path.starts_with("/api/desktop/") => {
                let parts: Vec<&str> = path.trim_start_matches("/api/desktop/").split('/').collect();
                let uuid = parts[0];
                if !desktop::valid_account_id(uuid) {
                    return Ok((404, r#"{"error":"not found"}"#.into(), "application/json"));
                }
                match (method.as_str(), parts.get(1).copied()) {
                    ("POST", Some("switch")) => app.start_desktop_switch(Some(uuid.to_string()))?,
                    ("POST", Some("rename")) => {
                        let label = body["label"].as_str().map(str::trim).filter(|s| !s.is_empty()).ok_or("name required")?;
                        let mut st = app.st.lock().unwrap();
                        let d = st.desktop.iter_mut().find(|d| d.uuid == uuid).ok_or("not found")?;
                        d.label = label.chars().take(60).collect();
                        app.save(&st);
                    }
                    ("DELETE", None) => {
                        if app.desktop_dir.as_deref().and_then(desktop::signed_in).as_deref() == Some(uuid) {
                            return Err("Claude Desktop is signed in to this account now; switch away first".into());
                        }
                        app.desktop_store.forget(uuid)?;
                        let mut st = app.st.lock().unwrap();
                        st.desktop.retain(|d| d.uuid != uuid);
                        app.log(&mut st, "Claude Desktop: removed a saved account".into());
                        app.save(&st);
                    }
                    _ => return Ok((404, r#"{"error":"not found"}"#.into(), "application/json")),
                }
            }
            _ => {
                let parts: Vec<&str> = path.trim_start_matches("/api/accounts/").split('/').collect();
                if !path.starts_with("/api/accounts/") || parts.is_empty() {
                    return Ok((404, r#"{"error":"not found"}"#.into(), "application/json"));
                }
                let id = parts[0];
                // activate takes the lock itself, around a check with Anthropic.
                if method == "POST" && parts.get(1) == Some(&"activate") {
                    app.activate(id, "manual")?;
                    return Ok((200, app.public_state(), "application/json"));
                }
                let mut st = app.st.lock().unwrap();
                let idx = st.accounts.iter().position(|a| a.id == id).ok_or("not found")?;
                match (method.as_str(), parts.get(1).copied()) {
                    ("POST", Some("toggle")) => {
                        st.accounts[idx].enabled = body["enabled"].as_bool().unwrap_or(true);
                        app.save(&st);
                    }
                    ("DELETE", None) => {
                        // Claude Code still holds it: it would come straight back as new.
                        if st.active_id.as_deref() == Some(id) {
                            return Err("Claude Code is signed in to this account now; switch to another first".into());
                        }
                        let a = st.accounts.remove(idx);
                        app.log(&mut st, format!("Removed {}", a.email));
                        app.save(&st);
                    }
                    _ => return Ok((404, r#"{"error":"not found"}"#.into(), "application/json")),
                }
            }
        }
        Ok((200, app.public_state(), "application/json"))
    })();

    let (code, text, ctype) = result.unwrap_or_else(|e| (400, json!({ "error": e }).to_string(), "application/json"));
    let resp = tiny_http::Response::from_string(text)
        .with_status_code(code)
        .with_header(tiny_http::Header::from_bytes("Content-Type", ctype).unwrap())
        .with_header(tiny_http::Header::from_bytes("Cache-Control", "no-store").unwrap());
    let _ = rq.respond(resp);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let demo = args.iter().any(|a| a == "--demo");
    let no_open = args.iter().any(|a| a == "--no-open");
    let port: u16 = std::env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(4777);

    let config_dir = std::env::var_os("CLAUDE_CONFIG_DIR").map(PathBuf::from);
    let claude_dir = config_dir.clone().unwrap_or_else(|| home().join(".claude"));
    let data_dir = std::env::var_os("CUD_DATA_DIR").map(PathBuf::from).unwrap_or_else(|| home().join(".claude-usage-dashboard"));

    // As a SessionStart hook: silent, and never fails the session it runs in.
    if args.iter().any(|a| a == "--note-session") {
        if std::env::var("CLAUDE_CODE_ENTRYPOINT").as_deref() == Ok("claude-desktop") {
            if let (Ok(u), Ok(e)) = (std::env::var("CLAUDE_CODE_ACCOUNT_UUID"), std::env::var("CLAUDE_CODE_USER_EMAIL")) {
                note_session(&data_dir, &u, &e);
            }
        }
        return;
    }

    let app = Arc::new(App {
        st: Mutex::new(State::default()),
        demo,
        data_file: data_dir.join("state.json"),
        creds_file: claude_dir.join(".credentials.json"),
        claude_json: match &config_dir {
            Some(d) => d.join(".claude.json"),
            None => home().join(".claude.json"),
        },
        keychain: cfg!(target_os = "macos") && std::env::var_os("CUD_NO_KEYCHAIN").is_none(),
        http: ureq::AgentBuilder::new().timeout(Duration::from_secs(20)).build(),
        polling: AtomicBool::new(false),
        last_poll: AtomicI64::new(0),
        seq: AtomicU64::new(0),
        port,
        desktop_dir: if demo { None } else { desktop::find_data_dir() },
        desktop_store: desktop::Store { root: data_dir.join("desktop") },
        switching: AtomicBool::new(false),
        login: Mutex::new(None),
        desktop_expect: Mutex::new(None),
    });

    {
        let mut st = app.st.lock().unwrap();
        if demo {
            *st = demo_state(&app);
        } else if let Some(s) = fs::read_to_string(&app.data_file).ok().and_then(|s| serde_json::from_str(&s).ok()) {
            *st = s;
        }
    }

    let server = match tiny_http::Server::http(("127.0.0.1", port)) {
        Ok(s) => s,
        Err(e) => {
            // Most likely it is already running: just open it.
            eprintln!("Port {port} busy ({e}); opening the existing dashboard.");
            if !no_open {
                open_browser(&format!("http://127.0.0.1:{port}"));
            }
            return;
        }
    };
    let url = format!("http://127.0.0.1:{port}");
    println!("Claude usage dashboard{}: {url}", if demo { " (demo)" } else { "" });
    if !demo {
        println!("Data: {}", app.data_file.display());
        println!("Claude Code login: {}", if app.keychain { "macOS Keychain".into() } else { app.creds_file.display().to_string() });
        println!("Claude Desktop data: {}", app.desktop_dir.as_ref().map(|d| d.display().to_string()).unwrap_or_else(|| "not found".into()));
    }
    println!("Keep this window open. Ctrl+C to stop.");
    if !no_open {
        open_browser(&url);
    }

    let poller = app.clone();
    thread::spawn(move || loop {
        poller.poll_all(false);
        let start = now();
        let mut last_sync = start;
        loop {
            thread::sleep(Duration::from_secs(1));
            // Each account has its own next check (next_check_at); look for due ones.
            if now() - start >= 5_000 {
                break;
            }
            // Notice a fresh /login (or Desktop sign-in) quickly instead of waiting a whole poll cycle.
            if !poller.demo && now() - last_sync >= 10_000 {
                last_sync = now();
                let before = poller.st.lock().unwrap().accounts.len();
                poller.sync_active();
                let mut st = poller.st.lock().unwrap();
                poller.sync_desktop(&mut st);
                if st.accounts.len() != before {
                    break;
                }
            }
        }
    });

    for rq in server.incoming_requests() {
        let app = app.clone();
        thread::spawn(move || handle(&app, rq));
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn checks_less_when_far_from_a_limit_and_not_at_all_while_one_is_full() {
        use super::{next_check_at, Usage, Win};
        let w = |u, r| Win { u, resets_at: r };
        let at = |five, week| Usage { five, week, fetched_at: 0 };
        let (base, t, h) = (120_000, 1_000_000, 3_600_000);
        // New account, little used: every base.
        assert_eq!(next_check_at(&at(w(5, None), w(10, None)), base, t), t + 120_000);
        // Nearer a limit (either window): more often.
        assert_eq!(next_check_at(&at(w(72, None), w(10, None)), base, t), t + 60_000);
        assert_eq!(next_check_at(&at(w(10, None), w(90, None)), base, t), t + 30_000);
        assert_eq!(next_check_at(&at(w(96, None), w(10, None)), base, t), t + 15_000);
        assert_eq!(next_check_at(&at(w(99, None), w(10, None)), 60_000, t), t + 15_000, "never under 15 s");
        // Weekly full, resets in 3 h: nothing until then (the 5-hour figure cannot help).
        assert_eq!(next_check_at(&at(w(0, Some(t + 2 * h)), w(100, Some(t + 3 * h))), base, t), t + 3 * h + 60_000);
        // 5-hour full: until it resets.
        assert_eq!(next_check_at(&at(w(100, Some(t + h)), w(40, Some(t + 50 * h))), base, t), t + h + 60_000);
        // Full but its reset time unknown or past: ask again soon, as near a limit.
        assert_eq!(next_check_at(&at(w(100, None), w(40, None)), base, t), t + 15_000);
        assert_eq!(next_check_at(&at(w(100, Some(t - 1)), w(40, None)), base, t), t + 15_000);
    }

    #[test]
    fn notes_desktop_session_names() {
        let d = std::env::temp_dir().join(format!("cud-names-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let u = "fb2bda4d-0000-4000-8000-000000000001";
        assert_eq!(super::noted_email(&d, u), None);
        assert!(super::note_session(&d, u, "a@example.com"));
        assert_eq!(super::noted_email(&d, u).as_deref(), Some("a@example.com"));
        assert!(super::note_session(&d, u, "b@example.com"), "a new email replaces the old");
        assert_eq!(super::noted_email(&d, u).as_deref(), Some("b@example.com"));
        assert!(!super::note_session(&d, "../../evil", "a@example.com"), "uuid is a file name: checked");
        assert!(!super::note_session(&d, u, "not an email"));
        assert_eq!(super::noted_email(&d, "../x"), None);
    }

    #[test]
    fn backs_off_after_429() {
        use super::{backoff_ms, retry_after_ms};
        assert_eq!(retry_after_ms(Some("0")), Some(0));
        assert_eq!(retry_after_ms(Some("120")), Some(120_000));
        assert_eq!(retry_after_ms(Some("Wed, 21 Oct 2026 07:28:00 GMT")), None);
        // Retry-After 0 (what Anthropic sent on 2026-10-28) names no wait: 5 min, doubling.
        let mins: Vec<i64> = (1..=6).map(|n| backoff_ms(Some(0), n) / 60_000).collect();
        assert_eq!(mins, vec![5, 10, 20, 40, 40, 40]);
        assert_eq!(backoff_ms(Some(90_000), 3), 90_000, "a named wait is kept");
        assert_eq!(backoff_ms(Some(5 * 3_600_000), 1), 3_600_000, "at most an hour");
    }

    #[test]
    fn iso() {
        assert_eq!(super::parse_iso("2025-10-28T11:40:00.123456+00:00"), Some(1761651600123));
        assert_eq!(super::parse_iso("2025-10-28T18:40:00+07:00"), Some(1761651600000));
        assert_eq!(super::parse_iso("2024-02-29T00:00:00Z"), Some(1709164800000));
    }
}
