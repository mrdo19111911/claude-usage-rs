// /api/gemini/* for the dashboard page: save the key, pick the model, remove, watch the
// clipboard for a copied key (watch.rs), open a window running cgemini (open.rs). Each
// answers with the whole public state, as the other routes do.

use crate::*;
use super::setup::{self, By};
use super::{key, models, path, script, upstream, watch, Config, FAST_MODEL, MODEL};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;

type Answer = Result<(u16, String, &'static str), String>;

// Some(answer) when `path` is one of these, else None. Same answer shape as server.rs.
pub(crate) fn api(app: &Arc<App>, method: &str, path: &str, body: &Value) -> Option<Answer> {
    let run: fn(&Arc<App>, &Value) -> Result<(), String> = match (method, path) {
        ("POST", "/api/gemini/key") => save_key,
        ("POST", "/api/gemini/model") => set_model,
        ("DELETE", "/api/gemini") => remove,
        ("POST", "/api/gemini/watch") => watch,
        ("POST", "/api/gemini/open") => super::open::open,
        _ => return None,
    };
    // The demo must not check keys or touch the real PATH and files.
    if app.demo {
        return Some(Err("Not in the demo".into()));
    }
    Some(run(app, body).map(|_| (200, app.public_state(), "application/json")))
}

// What /api/state shows of it (never the key, nor the token): null, or {keyEnd, model,
// fastModel, models, command}.
pub(crate) fn public(st: &State) -> Value {
    let Some(c) = &st.gemini else { return Value::Null };
    let chars: Vec<char> = c.key.chars().collect();
    let end: String = chars[chars.len().saturating_sub(4)..].iter().collect();
    json!({ "keyEnd": end, "model": c.model, "fastModel": c.fast_model, "models": models::text_models(&c.models), "command": "cgemini" })
}

// A fresh random token for cgemini (32 bytes, URL-safe base64: safe in a batch line too).
fn new_token() -> String {
    let mut b = [0u8; 32];
    getrandom::getrandom(&mut b).expect("system random numbers");
    format!("cud-{}", URL_SAFE_NO_PAD.encode(b))
}

fn save_key(app: &Arc<App>, body: &Value) -> Result<(), String> {
    save(app, body["key"].as_str().unwrap_or(""), &upstream::list_models)
}

// Copy to add (watch.rs): the dialog is open (on) or closed (off).
fn watch(app: &Arc<App>, body: &Value) -> Result<(), String> {
    match body["on"].as_bool() {
        Some(true) => {
            if let Some(run) = app.gemini_watch.on(now()) {
                watch::start(app, run);
            }
        }
        Some(false) => app.gemini_watch.off(),
        None => return Err("on: true or false".into()),
    }
    Ok(())
}

// A key pasted on the page or copied in AI Studio (watch.rs, By::Watch): checked with Google
// (`check`: upstream::list_models, or a test's), then cgemini written, put on PATH and the
// key saved, in that order and one save at a time (setup.rs).
pub(super) type Check<'a> = &'a dyn Fn(&str) -> Result<Vec<String>, String>;

pub(super) fn save(app: &Arc<App>, key: &str, check: Check) -> Result<(), String> {
    save_by(app, key, check, By::Page)
}

pub(super) fn save_by(app: &Arc<App>, key: &str, check: Check, by: By) -> Result<(), String> {
    let key = key.trim().to_string();
    // The same rule as a copied key's (key.rs): it goes into a header.
    if !key::fits(&key) {
        return Err("Paste the API key from Google AI Studio".into());
    }
    let all = check(&key)?;
    let cfg = Config {
        model: models::best(&all, MODEL),
        fast_model: models::best(&all, FAST_MODEL),
        key,
        token: new_token(),
        models: all,
        saved_at: now(),
    };
    setup::commit(app, cfg, by)
}

fn set_model(app: &Arc<App>, body: &Value) -> Result<(), String> {
    let _one = setup::lock();
    let cfg = {
        let mut st = app.st.lock().unwrap();
        let c = st.gemini.as_mut().ok_or("No Gemini key is saved")?;
        let known = |m: &str| c.models.iter().any(|x| x == m);
        // Only what is asked for is checked: a saved default the key lacks stays as it is.
        for m in [&body["model"], &body["fastModel"]].into_iter().filter_map(Value::as_str) {
            if !known(m) {
                return Err(format!("{m} is not one of this key's models"));
            }
        }
        let model = body["model"].as_str().unwrap_or(&c.model).to_string();
        let fast = body["fastModel"].as_str().unwrap_or(&c.fast_model).to_string();
        (c.model, c.fast_model) = (model, fast);
        let c = c.clone();
        app.save(&st);
        c
    };
    // The model is one of the script's variables.
    script::write(app, &cfg).map_err(|e| format!("cgemini.cmd could not be written: {e}"))
}

fn remove(app: &Arc<App>, _body: &Value) -> Result<(), String> {
    let _one = setup::lock();
    {
        let mut st = app.st.lock().unwrap();
        if st.gemini.take().is_none() {
            return Ok(());
        }
        app.log(&mut st, "Gemini: key removed, with the cgemini command".into());
        app.save(&st);
    }
    script::remove(app);
    path::remove(&script::bin_dir(app))
}
