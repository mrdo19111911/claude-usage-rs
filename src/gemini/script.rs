// cgemini.cmd in %USERPROFILE%\.claude-usage-dashboard\bin (next to state.json): sets
// Claude Code's variables for this app's port and token, runs `claude`, and lets nothing
// leak into the terminal afterwards (setlocal ... endlocal).

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

pub(crate) fn script_text(port: u16, cfg: &Config) -> String {
    let base = format!("http://127.0.0.1:{port}");
    let (model, fast) = (safe(&cfg.model), safe(&cfg.fast_model));
    let mut l: Vec<String> = vec![
        "@echo off".into(),
        "rem Written by Claude Usage at each start (its port changes): edits here are lost.".into(),
        "setlocal".into(),
        format!("curl.exe -sf -o nul -m 2 {base}/v1/ping"),
        "if errorlevel 1 (".into(),
        "  echo Claude Usage is not running: start it from the Start menu.".into(),
        "  exit /b 1".into(),
        ")".into(),
        format!("set \"ANTHROPIC_BASE_URL={base}\""),
        format!("set \"ANTHROPIC_AUTH_TOKEN={}\"", safe(&cfg.token)),
        // Signed in to claude.ai, Claude Code sends its login instead of the token above.
        format!("set \"ANTHROPIC_CUSTOM_HEADERS=x-cud-token: {}\"", safe(&cfg.token)),
        // A key of the user's own would be sent instead of the token, and asked about.
        "set ANTHROPIC_API_KEY=".into(),
        "set \"NO_PROXY=127.0.0.1,localhost\"".into(),
    ];
    for v in ["ANTHROPIC_MODEL", "ANTHROPIC_DEFAULT_OPUS_MODEL", "ANTHROPIC_DEFAULT_SONNET_MODEL", "ANTHROPIC_DEFAULT_FABLE_MODEL"] {
        l.push(format!("set \"{v}={model}\""));
    }
    for v in ["ANTHROPIC_DEFAULT_HAIKU_MODEL", "ANTHROPIC_SMALL_FAST_MODEL"] {
        l.push(format!("set \"{v}={fast}\""));
    }
    for v in [
        "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1",
        "CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS=1",
        "CLAUDE_CODE_ATTRIBUTION_HEADER=0",
        "CLAUDE_CODE_DISABLE_UNKNOWN_MODEL_WINDOW_ENFORCEMENT=1",
        "ENABLE_TOOL_SEARCH=false",
    ] {
        l.push(format!("set \"{v}\""));
    }
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
    l.push(r#"call "%CUD_CLAUDE%" %*"#.into());
    // %ERRORLEVEL% is read before endlocal runs: claude's own exit code goes out.
    l.push("endlocal & exit /b %ERRORLEVEL%".into());
    l.iter().map(|s| format!("{s}\r\n")).collect()
}

pub(crate) fn write(app: &App, cfg: &Config) -> std::io::Result<()> {
    fs::create_dir_all(bin_dir(app))?;
    write_private(&script_path(app), &script_text(app.port, cfg))
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
