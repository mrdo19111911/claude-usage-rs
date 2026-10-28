// Claude usage dashboard: theo dõi % usage 5 giờ / tuần của nhiều tài khoản Claude
// và tự chuyển login của Claude Code sang tài khoản còn dư khi chạm ngưỡng.
// Chạy file là xong: tự mở trình duyệt, tự nhận tài khoản mỗi lần bạn /login.

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
}

#[derive(Serialize, Deserialize, Clone)]
struct Event {
    t: i64,
    msg: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct State {
    settings: Settings,
    accounts: Vec<Account>,
    active_id: Option<String>,
    #[serde(default)]
    events: Vec<Event>,
}

impl Default for State {
    fn default() -> Self {
        State {
            settings: Settings { threshold: 95, auto_switch: true, poll_seconds: 120 },
            accounts: vec![],
            active_id: None,
            events: vec![],
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

    fn write_oauth_account(&self, oa: &Value) {
        if oa.is_null() {
            return;
        }
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

    // Work out which stored account Claude Code uses now. Picks up tokens Claude Code
    // refreshed on its own, and adds a login we have not seen yet as a new account.
    fn sync_active(&self, st: &mut State) {
        if self.demo {
            return;
        }
        let Some(live) = self.read_live() else {
            st.active_id = None;
            return;
        };
        let oa = self.read_oauth_account();
        let uuid = oa.get("accountUuid").and_then(Value::as_str).unwrap_or("");
        let email = oa.get("emailAddress").and_then(Value::as_str).unwrap_or("");
        let found = st.accounts.iter().position(|a| a.creds.get("refreshToken") == live.get("refreshToken"))
            .or_else(|| st.accounts.iter().position(|a| !uuid.is_empty() && a.oauth_account.get("accountUuid").and_then(Value::as_str) == Some(uuid)))
            .or_else(|| st.accounts.iter().position(|a| !email.is_empty() && a.email == email));
        match found {
            Some(i) => {
                let a = &mut st.accounts[i];
                if a.creds.get("accessToken") != live.get("accessToken") {
                    a.creds = live.clone();
                    a.error = None;
                }
                if !oa.is_null() {
                    a.oauth_account = oa;
                }
                st.active_id = Some(a.id.clone());
            }
            None => {
                let email = if email.is_empty() {
                    let rt = live.get("refreshToken").and_then(Value::as_str).unwrap_or("");
                    format!("account-{}", &rt[rt.len().saturating_sub(6)..])
                } else {
                    email.to_string()
                };
                let id = self.new_id();
                st.accounts.push(Account {
                    id: id.clone(),
                    email: email.clone(),
                    plan: plan_label(&live),
                    enabled: true,
                    creds: live,
                    oauth_account: oa,
                    usage: None,
                    history: vec![],
                    error: None,
                    raw: Value::Null,
                });
                st.active_id = Some(id);
                self.log(st, format!("Added {email} (new Claude Code login)"));
                self.save(st);
            }
        }
    }

    // ----- Anthropic API -----
    fn refresh(&self, creds: &Value) -> Result<Value, String> {
        let rt = creds.get("refreshToken").and_then(Value::as_str).ok_or("no refresh token")?;
        let mut last = String::from("refresh failed");
        for url in TOKEN_URLS {
            let r = self.http.post(url).send_json(json!({
                "grant_type": "refresh_token", "refresh_token": rt, "client_id": CLIENT_ID
            }));
            match r {
                Ok(resp) => {
                    let j: Value = resp.into_json().map_err(|e| e.to_string())?;
                    let mut c = creds.clone();
                    c["accessToken"] = j["access_token"].clone();
                    if j["refresh_token"].is_string() {
                        c["refreshToken"] = j["refresh_token"].clone();
                    }
                    c["expiresAt"] = json!(now() + j["expires_in"].as_i64().unwrap_or(3600) * 1000);
                    return Ok(c);
                }
                Err(ureq::Error::Status(404, _)) => continue,
                Err(ureq::Error::Status(code, resp)) => {
                    let j: Value = resp.into_json().unwrap_or(Value::Null);
                    let why = j["error_description"].as_str().or(j["error"].as_str()).unwrap_or("");
                    return Err(format!("refresh failed ({code}): {why} — /login this account again"));
                }
                Err(e) => last = e.to_string(),
            }
        }
        Err(last)
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

    // Returns the usage JSON and, if tokens were refreshed, the new creds.
    fn fetch_usage(&self, creds: &Value) -> Result<(Value, Option<Value>), String> {
        let mut c = creds.clone();
        let mut changed = false;
        if c.get("expiresAt").and_then(Value::as_i64).unwrap_or(0) - 60_000 < now() {
            c = self.refresh(&c)?;
            changed = true;
        }
        let j = match self.get_usage(&c) {
            Ok(j) => j,
            Err(ureq::Error::Status(401, _)) if !changed => {
                c = self.refresh(&c)?;
                changed = true;
                self.get_usage(&c).map_err(|e| e.to_string())?
            }
            Err(ureq::Error::Status(code, _)) => return Err(format!("usage HTTP {code}")),
            Err(e) => return Err(e.to_string()),
        };
        Ok((j, changed.then_some(c)))
    }

    fn poll_account(&self, id: &str) {
        let creds = {
            let st = self.st.lock().unwrap();
            match st.accounts.iter().find(|a| a.id == id) {
                Some(a) => a.creds.clone(),
                None => return,
            }
        };
        let res = self.fetch_usage(&creds);
        let mut st = self.st.lock().unwrap();
        let is_active = st.active_id.as_deref() == Some(id);
        let Some(a) = st.accounts.iter_mut().find(|a| a.id == id) else { return };
        match res {
            Ok((j, new_creds)) => {
                if let Some(c) = new_creds {
                    // The active account's token also lives in Claude Code: keep both in step.
                    if is_active {
                        let _ = self.write_live(&c);
                    }
                    a.creds = c;
                }
                let five = to_win(&j["five_hour"]);
                let week = to_win(&j["seven_day"]);
                let t = now();
                a.history.retain(|p| p.0 > t - 8 * 24 * HOUR);
                a.history.push((t, five.u, week.u));
                a.usage = Some(Usage { five, week, fetched_at: t });
                a.error = None;
                a.raw = j;
            }
            Err(e) => a.error = Some(e),
        }
        self.save(&st);
    }

    fn poll_all(&self) {
        if self.demo || self.polling.swap(true, Ordering::SeqCst) {
            return;
        }
        let ids: Vec<String> = {
            let mut st = self.st.lock().unwrap();
            self.sync_active(&mut st);
            st.accounts.iter().map(|a| a.id.clone()).collect()
        };
        for id in ids {
            self.poll_account(&id);
        }
        let mut st = self.st.lock().unwrap();
        self.auto_switch(&mut st);
        self.save(&st);
        self.last_poll.store(now(), Ordering::Relaxed);
        self.polling.store(false, Ordering::SeqCst);
    }

    // ----- switching -----
    fn activate(&self, st: &mut State, id: &str, reason: &str) -> Result<(), String> {
        self.sync_active(st); // save fresh tokens of the outgoing account first
        let acc = st.accounts.iter().find(|a| a.id == id).ok_or("not found")?.clone();
        if !self.demo {
            self.write_live(&acc.creds)?;
            self.write_oauth_account(&acc.oauth_account);
        }
        let from = st.active_id.as_ref().and_then(|f| st.accounts.iter().find(|a| &a.id == f)).map(|a| a.email.clone());
        st.active_id = Some(acc.id.clone());
        self.log(st, format!("Switched {} → {} ({reason})", from.unwrap_or("(none)".into()), acc.email));
        self.save(st);
        Ok(())
    }

    fn auto_switch(&self, st: &mut State) {
        if !st.settings.auto_switch {
            return;
        }
        let th = st.settings.threshold;
        let Some(active) = st.active_id.as_ref().and_then(|id| st.accounts.iter().find(|a| &a.id == id)).cloned() else { return };
        let Some(u) = &active.usage else { return };
        if u.five.u < th && u.week.u < th {
            return;
        }
        let room = |a: &Account| a.usage.as_ref().map(|u| (100 - u.five.u).min(100 - u.week.u)).unwrap_or(0);
        let next = st.accounts.iter()
            .filter(|a| a.enabled && a.id != active.id && a.error.is_none())
            .filter(|a| a.usage.as_ref().is_some_and(|u| u.five.u < th && u.week.u < th))
            .max_by_key(|a| room(a))
            .map(|a| a.id.clone());
        let which = if u.five.u >= th { format!("5-hour {}%", u.five.u) } else { format!("weekly {}%", u.week.u) };
        match next {
            Some(id) => {
                if let Err(e) = self.activate(st, &id, &format!("auto: {which}")) {
                    self.log(st, format!("Auto-switch failed: {e}"));
                }
            }
            None => {
                let msg = format!("{} hit {which} but no other account has room", active.email);
                if st.events.first().map(|e| &e.msg) != Some(&msg) {
                    self.log(st, msg);
                }
            }
        }
    }

    fn public_state(&self) -> String {
        let st = self.st.lock().unwrap();
        let t = now();
        let accounts: Vec<Value> = st.accounts.iter().map(|a| json!({
            "id": a.id, "email": a.email, "plan": a.plan, "enabled": a.enabled,
            "usage": a.usage, "error": a.error, "raw": a.raw,
            "history": a.history.iter().filter(|p| p.0 > t - 6 * HOUR).collect::<Vec<_>>(),
        })).collect();
        json!({
            "now": t,
            "lastPollAt": self.last_poll.load(Ordering::Relaxed),
            "polling": self.polling.load(Ordering::Relaxed),
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

fn open_browser(url: &str) {
    let _ = if cfg!(windows) {
        Command::new("cmd").args(["/C", "start", "", url]).spawn()
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
        email: email.into(),
        plan: "Max 20x".into(),
        enabled: true,
        creds: json!({}),
        oauth_account: Value::Null,
        usage: Some(Usage { five: Win { u: five, resets_at: five_reset }, week: Win { u: week, resets_at: Some(t + week_reset_h * HOUR) }, fetched_at: t - 12_000 }),
        history: hist,
        error: None,
        raw: Value::Null,
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
fn handle(app: &App, mut rq: tiny_http::Request) {
    let method = rq.method().as_str().to_string();
    let path = rq.url().split('?').next().unwrap_or("").to_string();
    let mut body = String::new();
    let _ = rq.as_reader().read_to_string(&mut body);
    let body: Value = serde_json::from_str(&body).unwrap_or(json!({}));
    let from_page = rq.headers().iter().any(|h| h.field.equiv("X-CUD") && h.value.as_str() == "1");

    let result: Result<(u16, String, &str), String> = (|| {
        if method == "GET" && (path == "/" || path == "/index.html") {
            return Ok((200, INDEX.to_string(), "text/html; charset=utf-8"));
        }
        if method == "GET" && path == "/api/state" {
            return Ok((200, app.public_state(), "application/json"));
        }
        // Only accept writes from the page itself (blocks cross-site form posts).
        if method != "GET" && !from_page {
            return Ok((403, r#"{"error":"forbidden"}"#.into(), "application/json"));
        }
        match (method.as_str(), path.as_str()) {
            ("POST", "/api/refresh") | ("POST", "/api/import") => {
                app.poll_all();
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
            _ => {
                let parts: Vec<&str> = path.trim_start_matches("/api/accounts/").split('/').collect();
                if !path.starts_with("/api/accounts/") || parts.is_empty() {
                    return Ok((404, r#"{"error":"not found"}"#.into(), "application/json"));
                }
                let id = parts[0];
                let mut st = app.st.lock().unwrap();
                let idx = st.accounts.iter().position(|a| a.id == id).ok_or("not found")?;
                match (method.as_str(), parts.get(1).copied()) {
                    ("POST", Some("toggle")) => {
                        st.accounts[idx].enabled = body["enabled"].as_bool().unwrap_or(true);
                        app.save(&st);
                    }
                    ("POST", Some("activate")) => app.activate(&mut st, id, "manual")?,
                    ("DELETE", None) => {
                        let a = st.accounts.remove(idx);
                        if st.active_id.as_deref() == Some(id) {
                            st.active_id = None;
                        }
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
    }
    println!("Keep this window open. Ctrl+C to stop.");
    if !no_open {
        open_browser(&url);
    }

    let poller = app.clone();
    thread::spawn(move || loop {
        poller.poll_all();
        let start = now();
        loop {
            thread::sleep(Duration::from_secs(1));
            let secs = poller.st.lock().unwrap().settings.poll_seconds as i64;
            if now() - start >= secs * 1000 {
                break;
            }
            // Notice a fresh /login quickly instead of waiting a whole poll cycle.
            if !poller.demo && (now() - start) % 10_000 < 1000 {
                let mut st = poller.st.lock().unwrap();
                let before = st.accounts.len();
                poller.sync_active(&mut st);
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
    fn iso() {
        assert_eq!(super::parse_iso("2025-10-28T11:40:00.123456+00:00"), Some(1761651600123));
        assert_eq!(super::parse_iso("2025-10-28T18:40:00+07:00"), Some(1761651600000));
        assert_eq!(super::parse_iso("2024-02-29T00:00:00Z"), Some(1709164800000));
    }
}
