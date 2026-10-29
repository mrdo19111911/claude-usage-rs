// /api/gemini/* and what /api/state shows of the key (never the key itself).

use super::script::script_path;
use super::test_app::{app, config, temp_dir, KEY};
use super::{api, public};
use serde_json::{json, Value};

#[test]
fn public_shows_no_key() {
    let mut st = crate::State::default();
    assert_eq!(public(&st), Value::Null);
    st.gemini = Some(config());
    let p = public(&st);
    assert_eq!(p["keyEnd"], "wxyz");
    assert_eq!(p["model"], super::MODEL);
    assert_eq!(p["fastModel"], super::FAST_MODEL);
    assert_eq!(p["models"].as_array().unwrap().len(), 3);
    assert_eq!(p["command"], "cgemini");
    assert!(!p.to_string().contains(KEY));
    assert!(!p.to_string().contains(&config().token), "nor the token");
}

#[test]
fn state_for_the_page_has_no_key() {
    let a = app(&temp_dir("state"), 1, Some(config()));
    let s = a.public_state();
    assert!(s.contains("\"keyEnd\":\"wxyz\""), "{s}");
    assert!(!s.contains(KEY));
    assert!(!s.contains("AIzaFAKE"));
}

#[test]
fn leaves_other_paths_alone() {
    let a = app(&temp_dir("other"), 1, None);
    assert!(api(&a, "POST", "/api/settings", &json!({})).is_none());
    assert!(api(&a, "POST", "/api/geminix", &json!({})).is_none());
}

#[test]
fn refuses_an_empty_or_odd_key_without_asking_google() {
    let a = app(&temp_dir("key"), 1, None);
    for k in [json!({}), json!({ "key": "  " }), json!({ "key": "a b" }), json!({ "key": "x\r\nHost: y" })] {
        let r = api(&a, "POST", "/api/gemini/key", &k).unwrap();
        assert!(r.is_err(), "{k}");
    }
    assert!(a.st.lock().unwrap().gemini.is_none());
}

#[test]
fn picks_a_model_of_the_key_and_rewrites_the_script() {
    let a = app(&temp_dir("model"), 4321, Some(config()));
    assert!(api(&a, "POST", "/api/gemini/model", &json!({ "model": "gemini-9" })).unwrap().is_err());
    let (code, text, _) = api(&a, "POST", "/api/gemini/model", &json!({ "model": "gemini-3.6-flash" })).unwrap().unwrap();
    assert_eq!(code, 200);
    let v: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(v["gemini"]["model"], "gemini-3.6-flash");
    assert_eq!(v["gemini"]["fastModel"], super::FAST_MODEL, "unchanged when not given");
    let script = std::fs::read_to_string(script_path(&a)).unwrap();
    assert!(script.contains("set \"ANTHROPIC_MODEL=gemini-3.6-flash\""));
    api(&a, "POST", "/api/gemini/model", &json!({ "model": super::MODEL, "fastModel": "gemini-3.6-flash" })).unwrap().unwrap();
    assert_eq!(a.st.lock().unwrap().gemini.as_ref().unwrap().fast_model, "gemini-3.6-flash");
}

#[test]
fn the_demo_changes_nothing() {
    let dir = temp_dir("demo");
    let a = app(&dir, 1, Some(config()));
    let mut demo = std::sync::Arc::into_inner(a).unwrap();
    demo.demo = true;
    let a = std::sync::Arc::new(demo);
    for (m, p, b) in [("POST", "/api/gemini/key", json!({ "key": "AIzaX" })), ("POST", "/api/gemini/model", json!({ "model": super::MODEL })), ("DELETE", "/api/gemini", json!({}))] {
        assert_eq!(api(&a, m, p, &b).unwrap().unwrap_err(), "Not in the demo", "{p}");
    }
    assert!(a.st.lock().unwrap().gemini.is_some());
    super::start(&a);
    assert!(!script_path(&a).exists(), "no script written in the demo");
}

#[test]
fn model_needs_a_key() {
    let a = app(&temp_dir("nokey"), 1, None);
    assert!(api(&a, "POST", "/api/gemini/model", &json!({ "model": super::MODEL })).unwrap().is_err());
}

#[test]
fn remove_forgets_the_key_and_the_script() {
    let a = app(&temp_dir("remove"), 1, Some(config()));
    super::script::write(&a, &config()).unwrap();
    let (code, text, _) = api(&a, "DELETE", "/api/gemini", &json!({})).unwrap().unwrap();
    assert_eq!(code, 200);
    assert!(text.contains("\"gemini\":null"));
    assert!(a.st.lock().unwrap().gemini.is_none());
    assert!(!script_path(&a).exists());
    let saved = std::fs::read_to_string(&a.data_file).unwrap();
    assert!(!saved.contains(KEY), "saved without it");
}
