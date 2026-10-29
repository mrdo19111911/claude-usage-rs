// cgemini.cmd in %USERPROFILE%\.claude-usage-dashboard\bin (next to state.json): sets
// Claude Code's variables for this app's port and token and its own profile (profile.rs),
// runs `claude`, and lets nothing leak into the terminal afterwards (setlocal ... endlocal).

use crate::*;
use super::Config;

pub(crate) fn bin_dir(app: &App) -> PathBuf {
    app.data_dir().join("bin")
}

pub(crate) fn script_path(app: &App) -> PathBuf {
    bin_dir(app).join("cgemini.cmd")
}

// Only what cannot change the meaning of a batch line (no & | < > ^ % " and the like).
fn safe(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_')).collect()
}

// Claude Code's switches to another provider (code.claude.com/docs/en/env-vars).
const PROVIDERS: [&str; 5] = [
    "CLAUDE_CODE_USE_BEDROCK",
    "CLAUDE_CODE_USE_VERTEX",
    "CLAUDE_CODE_USE_FOUNDRY",
    "CLAUDE_CODE_USE_MANTLE",
    "CLAUDE_CODE_USE_ANTHROPIC_AWS",
];

// Claude Code's variables for this app: in the script and in the profile's settings.json
// `env` (which Claude Code applies over the terminal's). Only safe characters.
pub(crate) fn env(port: u16, cfg: &Config) -> Vec<(&'static str, String)> {
    let (model, fast, token) = (safe(&cfg.model), safe(&cfg.fast_model), safe(&cfg.token));
    let mut v = vec![
        ("ANTHROPIC_BASE_URL", format!("http://127.0.0.1:{port}")),
        ("ANTHROPIC_AUTH_TOKEN", token.clone()),
        // Signed in to claude.ai, Claude Code sends its login instead of the token above.
        ("ANTHROPIC_CUSTOM_HEADERS", format!("x-cud-token: {token}")),
        // A key of the user's own would be sent instead of the token, and asked about.
        ("ANTHROPIC_API_KEY", String::new()),
        ("NO_PROXY", "127.0.0.1,localhost".into()),
    ];
    // Another provider chosen in the terminal or the user's settings would win over the
    // link. Empty: the script unsets them (`set NAME=`); in settings.json `env`, Claude
    // Code's docs (env-vars, "Precedence") treat "" as unset for provider selection. "0"
    // is not documented as off for these. The list as of 2026-10-30.
    for n in PROVIDERS {
        v.push((n, String::new()));
    }
    for n in ["ANTHROPIC_MODEL", "ANTHROPIC_DEFAULT_OPUS_MODEL", "ANTHROPIC_DEFAULT_SONNET_MODEL", "ANTHROPIC_DEFAULT_FABLE_MODEL"] {
        v.push((n, model.clone()));
    }
    for n in ["ANTHROPIC_DEFAULT_HAIKU_MODEL", "ANTHROPIC_SMALL_FAST_MODEL"] {
        v.push((n, fast.clone()));
    }
    for (n, x) in [
        ("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", "1"),
        ("CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS", "1"),
        ("CLAUDE_CODE_ATTRIBUTION_HEADER", "0"),
        ("CLAUDE_CODE_DISABLE_UNKNOWN_MODEL_WINDOW_ENFORCEMENT", "1"),
        ("ENABLE_TOOL_SEARCH", "false"),
    ] {
        v.push((n, x.into()));
    }
    v
}

pub(crate) fn script_text(port: u16, cfg: &Config) -> String {
    let mut l: Vec<String> = vec![
        "@echo off".into(),
        "rem Written by Claude Usage at each start (its port changes): edits here are lost.".into(),
        "setlocal".into(),
        // Never through a proxy: NO_PROXY is not set yet, and curl uses HTTP_PROXY or
        // ALL_PROXY for 127.0.0.1 too.
        format!(r#"curl.exe -sf --noproxy "*" -o nul -m 2 http://127.0.0.1:{port}/v1/ping"#),
        "if errorlevel 1 (".into(),
        "  echo Claude Usage is not running: start it from the Start menu.".into(),
        "  exit /b 1".into(),
        ")".into(),
    ];
    for (n, x) in env(port, cfg) {
        l.push(if x.is_empty() { format!("set {n}=") } else { format!("set \"{n}={x}\"") });
    }
    // Its own profile, next to this script's folder (profile.rs): no Claude login in it.
    l.push(r#"set "CLAUDE_CONFIG_DIR=%~dp0..\gemini-profile""#.into());
    // Claude Code on PATH, else where its own installer puts it (not always on PATH).
    for s in [
        r#"set "CUD_CLAUDE=claude""#,
        "where claude >nul 2>nul || set CUD_CLAUDE=",
        r#"if not defined CUD_CLAUDE if exist "%USERPROFILE%\.local\bin\claude.exe" set "CUD_CLAUDE=%USERPROFILE%\.local\bin\claude.exe""#,
        "if not defined CUD_CLAUDE (",
        "  echo Claude Code is not installed: see https://claude.com/claude-code",
        "  exit /b 1",
        ")",
    ] {
        l.push(s.into());
    }
    // `call`: claude may itself be a .cmd (npm), which would otherwise never return here.
    // No MCP servers: their tools were half of a request, and a free key's tokens a minute
    // are few (docs/specs/gemini.md).
    l.push(r#"call "%CUD_CLAUDE%" --strict-mcp-config %*"#.into());
    // %ERRORLEVEL% is read before endlocal runs: claude's own exit code goes out.
    l.push("endlocal & exit /b %ERRORLEVEL%".into());
    l.iter().map(|s| format!("{s}\r\n")).collect()
}

pub(crate) fn write(app: &App, cfg: &Config) -> std::io::Result<()> {
    fs::create_dir_all(bin_dir(app))?;
    write_private(&script_path(app), &script_text(app.port, cfg))?;
    super::profile::write(app, cfg)
}

pub(crate) fn remove(app: &App) {
    let _ = fs::remove_file(script_path(app));
}

// At start: the port is new, so the script is written again (if a key is saved).
pub(crate) fn start(app: &Arc<App>) {
    if app.demo {
        return;
    }
    let cfg = app.st.lock().unwrap().gemini.clone();
    if let Some(cfg) = cfg {
        if let Err(e) = write(app, &cfg) {
            let mut st = app.st.lock().unwrap();
            app.log(&mut st, format!("Gemini: could not write cgemini.cmd: {e}"));
        }
    }
}
