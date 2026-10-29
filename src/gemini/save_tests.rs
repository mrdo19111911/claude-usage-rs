// Saving a checked key: cgemini.cmd, its profile and PATH before state.json; a step that
// fails saves nothing and can be tried again; one rule for a key's length.

use super::api::save;
use super::path::FAIL;
use super::script::{bin_dir, script_path};
use super::test_app::{app, config, google_ok, temp_dir, KEY_A, TOKEN};
use super::key::key_in;
use super::watch::{save_for, Read, Source};
use std::cell::Cell;

// A clipboard of our own: one copy, a new sequence number for each copy.
struct Clip(Cell<u32>, &'static str);

impl Source for Clip {
    fn seq(&self) -> u32 {
        self.0.get()
    }
    fn read(&self) -> Read {
        Read::Text(self.1.into())
    }
}

#[test]
fn a_launcher_that_cannot_be_written_saves_no_key_and_the_watch_says_why() {
    let a = app(&temp_dir("setup-fails"), 1, None);
    std::fs::write(bin_dir(&a), "a file where the bin folder goes").unwrap();
    let run = a.gemini_watch.on(0).unwrap();
    let clip = Clip(Cell::new(1), KEY_A);
    let save = |k: &str| save_for(&a, run, k, &google_ok);
    let mut seen = None;
    assert!(a.gemini_watch.tick(run, 1, &mut seen, &clip, &save), "the watch goes on");
    assert!(a.st.lock().unwrap().gemini.is_none(), "no key saved: the page must not say ready");
    let w = a.gemini_watch.public();
    assert_eq!(w["state"], "error");
    assert!(w["error"].as_str().unwrap().contains("cgemini.cmd could not be written"), "{w}");
    // Tried again: the folder can be made now, and the key copied again.
    std::fs::remove_file(bin_dir(&a)).unwrap();
    clip.0.set(2);
    assert!(!a.gemini_watch.tick(run, 2, &mut seen, &clip, &save), "saved: the watch ends");
    assert_eq!(a.st.lock().unwrap().gemini.as_ref().unwrap().key, KEY_A);
    assert!(script_path(&a).exists());
}

#[test]
fn a_path_that_cannot_be_changed_keeps_the_key_before() {
    let a = app(&temp_dir("path-fails"), 1, Some(config()));
    super::script::write(&a, &config()).unwrap();
    FAIL.with(|f| f.set(true));
    let r = save(&a, KEY_A, &google_ok);
    FAIL.with(|f| f.set(false));
    assert!(r.unwrap_err().contains("PATH"));
    let c = a.st.lock().unwrap().gemini.clone().unwrap();
    assert_eq!((c.key.as_str(), c.token.as_str()), (config().key.as_str(), TOKEN), "the key before stays");
    let script = std::fs::read_to_string(script_path(&a)).unwrap();
    assert!(script.contains(&format!("x-cud-token: {TOKEN}")), "cgemini.cmd back to its token");
    // Tried again, PATH can be changed now.
    save(&a, KEY_A, &google_ok).unwrap();
    assert_eq!(a.st.lock().unwrap().gemini.as_ref().unwrap().key, KEY_A);
}

#[test]
fn a_path_that_cannot_be_changed_leaves_no_launcher_when_there_was_no_key() {
    let a = app(&temp_dir("path-fails-new"), 1, None);
    FAIL.with(|f| f.set(true));
    let r = save(&a, KEY_A, &google_ok);
    FAIL.with(|f| f.set(false));
    assert!(r.is_err());
    assert!(a.st.lock().unwrap().gemini.is_none());
    assert!(!script_path(&a).exists(), "no cgemini.cmd for a key not saved");
}

#[test]
fn every_key_the_watch_takes_can_be_saved() {
    let aq = |n: usize| format!("AQ.{}", "Ab1_.-".repeat(40).chars().take(n).collect::<String>());
    for n in [20, 197, 198, 200] {
        let key = aq(n);
        assert_eq!(key_in(&key).as_deref(), Some(key.as_str()), "the watch takes AQ. + {n}");
        let a = app(&temp_dir("long-key"), 1, None);
        assert_eq!(save(&a, &key, &google_ok), Ok(()), "AQ. + {n}");
    }
    let a = app(&temp_dir("too-long-key"), 1, None);
    let asked = Cell::new(false);
    let r = save(&a, &format!("{}x", aq(200)), &|k: &str| { asked.set(true); google_ok(k) });
    assert!(r.is_err() && !asked.get(), "one longer: refused without asking Google");
}
