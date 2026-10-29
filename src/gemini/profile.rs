// cgemini's own Claude Code profile (CLAUDE_CONFIG_DIR; docs/specs/gemini.md, "Its own
// Claude Code profile"): the user's settings with this app's link and token, their
// CLAUDE.md, onboarding done and no Claude login, so Claude Code opens at the prompt.

use crate::*;
use super::{script, Config};

// Settings about what this profile does not have (the plugins), or another key.
const DROPPED: [&str; 3] = ["enabledPlugins", "extraKnownMarketplaces", "apiKeyHelper"];

pub(crate) fn profile_dir(app: &App) -> PathBuf {
    app.data_dir().join("gemini-profile")
}

// The user's own Claude Code folder (~/.claude), where their login lives.
fn claude_dir(app: &App) -> &Path {
    app.creds_file.parent().unwrap_or(Path::new("."))
}

pub(crate) fn write(app: &App, cfg: &Config) -> std::io::Result<()> {
    let dir = profile_dir(app);
    fs::create_dir_all(&dir)?;
    let s = serde_json::to_string_pretty(&settings(app, cfg)).unwrap_or_default();
    write_private(&dir.join("settings.json"), &s)?;
    match fs::read(claude_dir(app).join("CLAUDE.md")) {
        Ok(md) => fs::write(dir.join("CLAUDE.md"), md)?,
        Err(_) => {
            let _ = fs::remove_file(dir.join("CLAUDE.md"));
        }
    }
    onboarded(&dir.join(".claude.json"))
}

// The user's settings.json, without DROPPED, its `env` with this app's variables on top.
fn settings(app: &App, cfg: &Config) -> Value {
    let mine = fs::read_to_string(claude_dir(app).join("settings.json")).ok();
    let mut s = match mine.and_then(|t| serde_json::from_str::<Value>(&t).ok()) {
        Some(Value::Object(m)) => m,
        _ => Default::default(),
    };
    for k in DROPPED {
        s.remove(k);
    }
    let mut env = match s.remove("env") {
        Some(Value::Object(m)) => m,
        _ => Default::default(),
    };
    for (n, x) in script::env(app.port, cfg) {
        env.insert(n.into(), Value::String(x));
    }
    s.insert("env".into(), Value::Object(env));
    Value::Object(s)
}

// Onboarding marked done once (with a theme, its first question); what Claude Code writes
// there itself (history, trusted folders, its own theme setting) is kept. Seen
// 2026-10-29: Claude Code drops `theme` from this file, so only the onboarding mark
// decides whether to write.
fn onboarded(path: &Path) -> std::io::Result<()> {
    let mut c = match fs::read_to_string(path).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()) {
        Some(Value::Object(m)) => m,
        _ => Default::default(),
    };
    if c.get("hasCompletedOnboarding") == Some(&Value::Bool(true)) {
        return Ok(());
    }
    c.insert("hasCompletedOnboarding".into(), Value::Bool(true));
    c.entry("theme").or_insert_with(|| Value::String("dark".into()));
    fs::write(path, Value::Object(c).to_string())
}
