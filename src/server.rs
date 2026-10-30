// The local web server: one function per request.

use crate::*;

// ---------- HTTP ----------
pub(crate) fn handle(app: &Arc<App>, mut rq: tiny_http::Request) {
    let method = rq.method().as_str().to_string();
    let url = rq.url().to_string();
    let path = url.split('?').next().unwrap_or("").to_string();
    // Claude Code (cgemini): its own checks (a token), its own answers (streamed).
    if path.starts_with("/v1/") {
        return gemini::serve(app, rq);
    }
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

    if let (true, "GET", Some(png)) = (host_ok, method.as_str(), web::image(&path)) {
        let resp = tiny_http::Response::from_data(png)
            .with_header(tiny_http::Header::from_bytes("Content-Type", "image/png").unwrap());
        let _ = rq.respond(resp);
        return;
    }
    // A download (Content-Disposition): a GET, so a plain link on the page works.
    if host_ok && method == "GET" && path == "/api/usage-log/export" {
        return usage_export::respond(app, rq);
    }
    let result: Result<(u16, String, &str), String> = (|| {
        if !host_ok {
            return Ok((403, r#"{"error":"forbidden host"}"#.into(), "application/json"));
        }
        if method == "GET" {
            if let Some((text, ctype)) = web::asset(&path) {
                return Ok((200, text.to_string(), ctype));
            }
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
        if let Some(answer) = gemini::api(app, &method, &path, &body) {
            return answer;
        }
        match (method.as_str(), path.as_str()) {
            ("POST", "/api/refresh") | ("POST", "/api/import") => {
                app.poll_all(true);
            }
            ("POST", "/api/exit") => {
                // After this answer has gone out.
                thread::spawn(|| {
                    thread::sleep(Duration::from_millis(300));
                    #[cfg(windows)]
                    tray::request_exit();
                    #[cfg(not(windows))]
                    std::process::exit(0);
                });
                return Ok((200, "{}".into(), "application/json"));
            }
            ("POST", "/api/settings") => {
                let mut st = app.st.lock().unwrap();
                if let Some(v) = body["threshold"].as_f64() {
                    st.settings.threshold = (v as i64).clamp(50, 100);
                }
                if let Some(v) = body["weekThreshold"].as_f64() {
                    st.settings.week_threshold = (v as i64).clamp(50, 100);
                }
                if let Some(v) = body["autoSwitch"].as_bool() {
                    st.settings.auto_switch = v;
                }
                if let Some(v) = body["pollSeconds"].as_f64() {
                    st.settings.poll_seconds = (v as u64).max(30);
                }
                if let Some(v) = body["autoUpdate"].as_bool() {
                    st.settings.auto_update = v;
                }
                if let Some(v) = body["shareDesktopSessions"].as_bool() {
                    st.settings.share_desktop_sessions = v;
                }
                app.save(&st);
            }
            // Both reach GitHub (the install downloads a setup): in the background; the
            // page follows "update" in /api/state.
            ("POST", "/api/update/check") => {
                if app.begin_check() {
                    let a = app.clone();
                    thread::spawn(move || a.finish_check());
                }
            }
            ("POST", "/api/update/install") => {
                app.begin_install()?;
                let a = app.clone();
                thread::spawn(move || a.finish_install());
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

