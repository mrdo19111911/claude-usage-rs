// The Gemini profile: a copy of the user's Claude Code settings with this app's link and
// token, onboarding done, no Claude login (docs/specs/gemini.md, "Its own Claude Code profile").

use super::profile::{profile_dir, write};
use super::script::bin_dir;
use super::test_app::{app, config, temp_dir, KEY, TOKEN};
use serde_json::{json, Value};
use std::fs;

fn read(p: std::path::PathBuf) -> Value {
    serde_json::from_str(&fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))).unwrap()
}

#[test]
fn is_next_to_the_script_folder() {
    let a = app(&temp_dir("prof-dir"), 4777, Some(config()));
    // cgemini.cmd finds it as %~dp0..\gemini-profile.
    assert_eq!(profile_dir(&a), bin_dir(&a).parent().unwrap().join("gemini-profile"));
}

#[test]
fn settings_are_the_users_with_our_link_and_token() {
    let dir = temp_dir("prof-set");
    let mine = json!({
        "hooks": {"Stop": [{"hooks": [{"type": "command", "command": "echo done"}]}]},
        "statusLine": {"type": "command", "command": "my-status"},
        "enabledPlugins": {"x@y": true}, "extraKnownMarketplaces": {"y": {}}, "apiKeyHelper": "get-key",
        "env": {"ANTHROPIC_BASE_URL": "https://api.z.ai/api/anthropic", "ANTHROPIC_AUTH_TOKEN": "zai", "MY_VAR": "1",
                "CLAUDE_CODE_USE_BEDROCK": "1", "CLAUDE_CODE_USE_VERTEX": "true"},
    });
    fs::write(dir.join("settings.json"), mine.to_string()).unwrap();
    let a = app(&dir, 4777, Some(config()));
    write(&a, &config()).unwrap();
    let s = read(profile_dir(&a).join("settings.json"));
    assert_eq!(s["hooks"], mine["hooks"]);
    assert_eq!(s["statusLine"], mine["statusLine"]);
    for gone in ["enabledPlugins", "extraKnownMarketplaces", "apiKeyHelper"] {
        assert!(s.get(gone).is_none(), "no {gone}");
    }
    let env = &s["env"];
    assert_eq!(env["ANTHROPIC_BASE_URL"], "http://127.0.0.1:4777");
    assert_eq!(env["ANTHROPIC_AUTH_TOKEN"], TOKEN);
    assert_eq!(env["ANTHROPIC_MODEL"], super::MODEL);
    assert_eq!(env["ANTHROPIC_SMALL_FAST_MODEL"], super::FAST_MODEL);
    assert_eq!(env["ANTHROPIC_API_KEY"], "");
    assert_eq!(env["MY_VAR"], "1", "the user's other variables stay");
    // Another provider would win over the link: "" is off for provider selection.
    for p in ["BEDROCK", "VERTEX", "FOUNDRY", "MANTLE", "ANTHROPIC_AWS"] {
        assert_eq!(env[format!("CLAUDE_CODE_USE_{p}")], "", "{p}");
    }
    assert!(!fs::read_to_string(profile_dir(&a).join("settings.json")).unwrap().contains(KEY), "never the Google key");
}

#[test]
fn without_user_settings_it_still_has_the_link() {
    let a = app(&temp_dir("prof-none"), 5123, Some(config()));
    write(&a, &config()).unwrap();
    assert_eq!(read(profile_dir(&a).join("settings.json"))["env"]["ANTHROPIC_BASE_URL"], "http://127.0.0.1:5123");
    assert!(!profile_dir(&a).join("CLAUDE.md").exists(), "none to copy");
}

#[test]
fn copies_claude_md_and_never_the_login() {
    let dir = temp_dir("prof-md");
    fs::write(dir.join("CLAUDE.md"), "# my rules\n").unwrap();
    fs::write(dir.join(".credentials.json"), "{\"claudeAiOauth\":{}}").unwrap();
    let a = app(&dir, 4777, Some(config()));
    write(&a, &config()).unwrap();
    assert_eq!(fs::read_to_string(profile_dir(&a).join("CLAUDE.md")).unwrap(), "# my rules\n");
    assert!(!profile_dir(&a).join(".credentials.json").exists(), "no Claude login in it");
}

#[test]
fn onboarding_is_done_and_claude_codes_own_notes_stay() {
    let a = app(&temp_dir("prof-onb"), 4777, Some(config()));
    write(&a, &config()).unwrap();
    let j = profile_dir(&a).join(".claude.json");
    let c = read(j.clone());
    assert_eq!((c["hasCompletedOnboarding"].clone(), c["theme"].clone()), (json!(true), json!("dark")));
    // Claude Code writes its own things there; the user picks another theme.
    fs::write(&j, json!({"hasCompletedOnboarding": true, "theme": "light", "projects": {"C:/x": {}}}).to_string()).unwrap();
    write(&a, &config()).unwrap();
    let c = read(j.clone());
    assert_eq!(c["theme"], "light");
    assert!(c["projects"]["C:/x"].is_object());
    // Claude Code drops `theme` from the file: left as it is, not written again.
    let theirs = json!({"hasCompletedOnboarding": true, "machineID": "m"}).to_string();
    fs::write(&j, &theirs).unwrap();
    write(&a, &config()).unwrap();
    assert_eq!(fs::read_to_string(&j).unwrap(), theirs);
}
