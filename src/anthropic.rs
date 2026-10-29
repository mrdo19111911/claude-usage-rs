// Anthropic's endpoints: token refresh, usage, profile.

use crate::*;

pub(crate) const CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";
pub(crate) const TOKEN_URLS: [&str; 2] = [
    "https://platform.claude.com/v1/oauth/token",
    "https://console.anthropic.com/v1/oauth/token",
];
pub(crate) const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";
// Whose a token is: ~/.claude.json's oauthAccount can name another account than the
// token in .credentials.json (seen on 2026-10-28), so identity comes from here.
pub(crate) const PROFILE_URL: &str = "https://api.anthropic.com/api/oauth/profile";

impl App {
    // ----- Anthropic API -----
    pub(crate) fn refresh(&self, creds: &Value) -> Result<Value, ApiErr> {
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

    pub(crate) fn get_usage(&self, creds: &Value) -> Result<Value, ureq::Error> {
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
    pub(crate) fn get_profile(&self, creds: &Value) -> Result<(String, String), ApiErr> {
        let j = self.get_profile_json(creds)?;
        let uuid = j["account"]["uuid"].as_str().ok_or(ApiErr::Other("profile without account".into()))?;
        Ok((uuid.to_string(), j["account"]["email"].as_str().unwrap_or("").to_string()))
    }

    pub(crate) fn get_profile_json(&self, creds: &Value) -> Result<Value, ApiErr> {
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
    pub(crate) fn fetch_usage(&self, creds: &Value, may_refresh: bool) -> Result<(Value, Option<Value>), ApiErr> {
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
    pub(crate) fn verify_login(&self, creds: &Value, uuid: Option<&str>) -> Result<Value, String> {
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
}

// A failed call to Anthropic. Auth: the login was refused. Slow: a 429, with the wait
// Retry-After named, if it did.
pub(crate) enum ApiErr {
    Auth(String),
    Slow(Option<i64>),
    Other(String),
}

impl ApiErr {
    pub(crate) fn text(&self) -> String {
        match self {
            ApiErr::Auth(s) | ApiErr::Other(s) => s.clone(),
            ApiErr::Slow(_) => "Anthropic asked to slow down (HTTP 429)".into(),
        }
    }
}

pub(crate) fn api_err(e: ureq::Error, what: &str) -> ApiErr {
    match e {
        ureq::Error::Status(401, _) => ApiErr::Auth(format!("{what} HTTP 401")),
        ureq::Error::Status(429, resp) => ApiErr::Slow(retry_after_ms(resp.header("Retry-After"))),
        ureq::Error::Status(code, _) => ApiErr::Other(format!("{what} HTTP {code}")),
        e => ApiErr::Other(e.to_string()),
    }
}

// Retry-After in seconds (the form Anthropic sends), as ms.
pub(crate) fn retry_after_ms(v: Option<&str>) -> Option<i64> {
    v?.trim().parse::<f64>().ok().filter(|s| s.is_finite() && *s >= 0.0).map(|s| (s * 1000.0) as i64)
}
