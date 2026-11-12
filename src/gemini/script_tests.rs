// cgemini.cmd: its text, and where it is written.

use super::script::{script_path, script_text, write};
use super::test_app::{app, config, temp_dir, KEY, TOKEN};

fn text() -> String {
    script_text(4777, &config())
}

fn line_of(t: &str, needle: &str) -> usize {
    t.lines().position(|l| l.contains(needle)).unwrap_or_else(|| panic!("no line with {needle:?} in\n{t}"))
}

#[test]
fn is_a_windows_batch_file_with_crlf_lines() {
    let t = text();
    assert!(t.starts_with("@echo off\r\n"));
    assert!(t.ends_with("\r\n"));
    assert_eq!(t.matches('\n').count(), t.matches("\r\n").count(), "no bare LF");
}

#[test]
fn checks_the_app_first() {
    let t = text();
    let ping = line_of(&t, "/v1/ping");
    assert!(line_of(&t, "Smartlog Claude Auto Switch is not running: start it from the Start menu.") > ping);
    assert!(line_of(&t, "exit /b 1") > ping);
    assert!(ping < line_of(&t, "ANTHROPIC_BASE_URL"));
}

// The ping runs before NO_PROXY is set; a proxy from HTTP_PROXY or ALL_PROXY must not
// carry it (curl uses them for 127.0.0.1 too).
#[test]
fn the_ping_never_goes_through_a_proxy() {
    let t = text();
    let ping = t.lines().find(|l| l.contains("/v1/ping")).unwrap();
    assert_eq!(ping, r#"curl.exe -sf --noproxy "*" -o nul -m 2 http://127.0.0.1:4777/v1/ping"#);
}

// Bedrock, Vertex and the like set in the terminal would win over the link: unset.
#[test]
fn clears_the_other_providers() {
    let t = text();
    let local = line_of(&t, "setlocal");
    for p in ["BEDROCK", "VERTEX", "FOUNDRY", "MANTLE", "ANTHROPIC_AWS"] {
        let v = format!("set CLAUDE_CODE_USE_{p}=\r\n");
        assert!(t.contains(&v), "missing {v:?} in\n{t}");
        assert!(line_of(&t, v.trim_end()) > local);
    }
}

#[test]
fn sets_the_variables_inside_setlocal() {
    let t = text();
    let local = line_of(&t, "setlocal");
    for v in [
        "set \"ANTHROPIC_BASE_URL=http://127.0.0.1:4777\"",
        &format!("set \"ANTHROPIC_AUTH_TOKEN={TOKEN}\""),
        &format!("set \"ANTHROPIC_CUSTOM_HEADERS=x-cud-token: {TOKEN}\""),
        "set ANTHROPIC_API_KEY=\r\n",
        "set \"NO_PROXY=127.0.0.1,localhost\"",
        "set \"ANTHROPIC_MODEL=gemini-3.5-flash-lite\"",
        "set \"ANTHROPIC_DEFAULT_OPUS_MODEL=gemini-3.5-flash-lite\"",
        "set \"ANTHROPIC_DEFAULT_SONNET_MODEL=gemini-3.5-flash-lite\"",
        "set \"ANTHROPIC_DEFAULT_FABLE_MODEL=gemini-3.5-flash-lite\"",
        "set \"ANTHROPIC_DEFAULT_HAIKU_MODEL=gemini-3.1-flash-lite\"",
        "set \"ANTHROPIC_SMALL_FAST_MODEL=gemini-3.1-flash-lite\"",
        "set \"CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1\"",
        "set \"CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS=1\"",
        "set \"CLAUDE_CODE_ATTRIBUTION_HEADER=0\"",
        "set \"CLAUDE_CODE_DISABLE_UNKNOWN_MODEL_WINDOW_ENFORCEMENT=1\"",
        "set \"ENABLE_TOOL_SEARCH=false\"",
    ] {
        assert!(t.contains(v), "missing {v:?}");
        assert!(line_of(&t, v.trim_end()) > local, "{v:?} before setlocal");
    }
}

#[test]
fn runs_claude_then_ends_the_local_scope() {
    let t = text();
    let run = line_of(&t, "call \"%CUD_CLAUDE%\" --strict-mcp-config %*");
    // Claude Code on PATH, else where its own installer puts it, else a plain message.
    let find = line_of(&t, "where claude >nul 2>nul");
    assert!(t.contains(r#"if exist "%USERPROFILE%\.local\bin\claude.exe" set "CUD_CLAUDE=%USERPROFILE%\.local\bin\claude.exe""#), "{t}");
    let none = line_of(&t, "echo Claude Code is not installed: see https://claude.com/claude-code");
    assert!(find < none && none < run);
    assert!(run > line_of(&t, "ENABLE_TOOL_SEARCH"));
    // Its own profile, next to the script's folder: no Claude login, onboarding done.
    let prof = line_of(&t, r#"set "CLAUDE_CONFIG_DIR=%~dp0..\gemini-profile""#);
    assert!(line_of(&t, "setlocal") < prof && prof < run);
    assert_eq!(t.trim_end().lines().last(), Some("endlocal & exit /b %ERRORLEVEL%"));
}

#[test]
fn never_holds_the_key_and_keeps_only_safe_characters() {
    assert!(!text().contains(KEY));
    let mut c = config();
    c.model = "gemini-x&calc.exe|\"%PATH%".into();
    let t = script_text(1, &c);
    assert!(t.contains("set \"ANTHROPIC_MODEL=gemini-xcalc.exePATH\""), "{t}");
}

#[test]
fn is_written_next_to_the_state_file() {
    let dir = temp_dir("script");
    let a = app(&dir, 4777, Some(config()));
    assert_eq!(script_path(&a), dir.join("bin").join("cgemini.cmd"));
    write(&a, &config()).unwrap();
    assert_eq!(std::fs::read_to_string(script_path(&a)).unwrap(), text());
}

#[test]
fn start_rewrites_it_for_the_new_port_only_with_a_key() {
    let dir = temp_dir("start");
    let a = app(&dir, 5123, None);
    super::start(&a);
    assert!(!script_path(&a).exists(), "no key: no script");
    a.st.lock().unwrap().gemini = Some(config());
    super::start(&a);
    assert!(std::fs::read_to_string(script_path(&a)).unwrap().contains("127.0.0.1:5123"));
}
