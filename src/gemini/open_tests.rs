// "Use this": a new console window running cgemini (docs/specs/gemini.md, "Onboarding").

use super::open::{args, opened};
use super::script::script_path;
use super::test_app::{app, config, temp_dir};
use serde_json::json;
use std::path::Path;

#[test]
fn the_window_runs_the_script_and_stays_open() {
    let s = Path::new(r"C:\Users\A B\.claude-usage-dashboard\bin\cgemini.cmd");
    assert_eq!(args(s), r#"/k "C:\Users\A B\.claude-usage-dashboard\bin\cgemini.cmd""#);
}

#[test]
fn opens_cgemini_from_the_home_folder() {
    let a = app(&temp_dir("open"), 4777, Some(config()));
    let r = super::api(&a, "POST", "/api/gemini/open", &json!({})).expect("a route");
    assert!(r.is_ok(), "{r:?}");
    let (dir, line) = opened().expect("a window opened");
    assert_eq!(line, args(&script_path(&a)));
    assert!(script_path(&a).exists(), "the script is there to run");
    assert_eq!(Some(dir.as_os_str()), std::env::var_os("USERPROFILE").as_deref(), "Claude Code starts in the home folder");
}

#[test]
fn without_a_key_nothing_opens() {
    let a = app(&temp_dir("open-none"), 4777, None);
    let r = super::api(&a, "POST", "/api/gemini/open", &json!({})).unwrap();
    assert_eq!(r.unwrap_err(), "Add a Gemini account first");
}
