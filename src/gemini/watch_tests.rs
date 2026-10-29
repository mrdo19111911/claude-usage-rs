// Copy to add: which clipboard text counts as a key, the watch's states, the route.

use super::test_app::{app, config, temp_dir};
use super::watch::{key_in, Read, Source, Watch, WATCH_MS};
use super::api;
use serde_json::{json, Value};
use std::cell::{Cell, RefCell};

const AIZA: &str = "AIzaSyA-bc_DEFghijKLMNopqrSTUvwxyz01234"; // AIza + 35

// A clipboard of our own: the text it holds and its sequence number; counts the reads.
struct Fake {
    seq: Cell<u32>,
    now: RefCell<Read>,
    reads: Cell<u32>,
}

impl Fake {
    fn new(r: Read) -> Fake {
        Fake { seq: Cell::new(1), now: RefCell::new(r), reads: Cell::new(0) }
    }
    fn copy(&self, r: Read) {
        self.seq.set(self.seq.get() + 1);
        *self.now.borrow_mut() = r;
    }
}

impl Source for Fake {
    fn seq(&self) -> u32 {
        self.seq.get()
    }
    fn read(&self) -> Read {
        self.reads.set(self.reads.get() + 1);
        self.now.borrow().clone()
    }
}

fn text(s: &str) -> Read {
    Read::Text(s.into())
}

#[test]
fn only_a_whole_google_key_counts() {
    assert_eq!(AIZA.len(), 39);
    assert_eq!(key_in(AIZA).as_deref(), Some(AIZA));
    assert_eq!(key_in(&format!("  {AIZA}\r\n")).as_deref(), Some(AIZA), "copied with a line end");
    let aq = |n: usize| format!("AQ.{}", "Ab1_.-".repeat(40).chars().take(n).collect::<String>());
    for n in [20, 21, 120, 200] {
        assert_eq!(key_in(&aq(n)), Some(aq(n)), "AQ. + {n}");
    }
    let no = [
        aq(19), aq(201), "AQ.".into(), "AIza".into(), String::new(), "   ".into(),
        AIZA[..38].into(), format!("{AIZA}x"), // 34 and 36 after AIza
        AIZA.replace("bc_D", "bc D"), AIZA.replace("bc_D", "bc\nD"), AIZA.replace("bc_D", "bc.D"),
        format!("{} {}", &aq(30)[..15], &aq(30)[15..]), format!("{}é", aq(30)),
        "aiza".to_string() + &AIZA[4..], "Hunter2!Correct-Horse-Battery".into(), format!("key: {AIZA}"),
    ];
    for t in no {
        assert_eq!(key_in(&t), None, "{t:?}");
    }
}

#[test]
fn on_extends_and_off_stops() {
    let w = Watch::default();
    assert_eq!(w.public(), Value::Null);
    let run = w.on(1000).expect("a first on starts a watcher");
    assert_eq!(w.public(), json!({ "state": "waiting", "error": null, "until": 1000 + WATCH_MS }));
    assert_eq!(w.on(5000), None, "a second on starts no second watcher");
    assert_eq!(w.public()["until"], 5000 + WATCH_MS);
    w.off();
    assert_eq!(w.public(), Value::Null);
    let mut seen = None;
    assert!(!w.tick(run, 6000, &mut seen, &Fake::new(text(AIZA)), &|_: &str| Ok(())), "stopped");
    assert!(w.on(7000).unwrap() > run, "a new run after an off");
}

#[test]
fn other_text_is_read_once_and_left_alone() {
    let w = Watch::default();
    let run = w.on(0).unwrap();
    let clip = Fake::new(text("my password 1234"));
    let saved = RefCell::new(vec![]);
    let save = |k: &str| { saved.borrow_mut().push(k.to_string()); Ok(()) };
    let mut seen = None;
    for t in [1, 2, 3] {
        assert!(w.tick(run, t, &mut seen, &clip, &save));
    }
    assert_eq!(clip.reads.get(), 1, "read only when the sequence number changes");
    assert!(saved.borrow().is_empty());
    assert_eq!(w.public()["state"], "waiting");
    clip.copy(Read::Other); // a picture, say
    assert!(w.tick(run, 4, &mut seen, &clip, &save));
    assert_eq!(clip.reads.get(), 2);
    assert!(saved.borrow().is_empty());
}

#[test]
fn a_key_is_checked_saved_and_the_watch_ends() {
    let w = Watch::default();
    let run = w.on(0).unwrap();
    let clip = Fake::new(text("hello"));
    let mut seen = None;
    let states = RefCell::new(vec![]);
    let save = |k: &str| { states.borrow_mut().push((k.to_string(), w.public()["state"].clone())); Ok(()) };
    assert!(w.tick(run, 1, &mut seen, &clip, &save));
    clip.copy(text(&format!("{AIZA}\n")));
    assert!(!w.tick(run, 2, &mut seen, &clip, &save), "done");
    assert_eq!(*states.borrow(), vec![(AIZA.to_string(), json!("checking"))]);
    assert_eq!(w.public(), Value::Null);
}

#[test]
fn a_refused_key_shows_the_error_and_waits_for_a_new_copy() {
    let w = Watch::default();
    let run = w.on(0).unwrap();
    let clip = Fake::new(text(AIZA));
    let tries = Cell::new(0);
    let save = |_: &str| { tries.set(tries.get() + 1); Err("API key not valid".to_string()) };
    let mut seen = None;
    assert!(w.tick(run, 1, &mut seen, &clip, &save));
    assert_eq!(w.public()["state"], "error");
    assert_eq!(w.public()["error"], "API key not valid");
    assert!(w.tick(run, 2, &mut seen, &clip, &save));
    assert_eq!(tries.get(), 1, "the same copy is not tried again");
    clip.copy(text(AIZA));
    let ok = |_: &str| Ok(());
    assert!(!w.tick(run, 3, &mut seen, &clip, &ok), "copied again: tried again");
}

#[test]
fn a_busy_clipboard_is_read_on_the_next_tick() {
    let w = Watch::default();
    let run = w.on(0).unwrap();
    let clip = Fake::new(Read::Busy);
    let mut seen = None;
    let ok = |_: &str| Ok(());
    assert!(w.tick(run, 1, &mut seen, &clip, &ok));
    assert!(w.tick(run, 2, &mut seen, &clip, &ok));
    assert_eq!(clip.reads.get(), 2, "the same sequence number, read again");
    *clip.now.borrow_mut() = text(AIZA);
    assert!(!w.tick(run, 3, &mut seen, &clip, &ok));
}

#[test]
fn ten_minutes_at_most_and_one_watcher_per_run() {
    let w = Watch::default();
    let old = w.on(0).unwrap();
    let ok = |_: &str| Ok(());
    let clip = Fake::new(text("x"));
    let mut seen = None;
    assert!(!w.tick(old, WATCH_MS, &mut seen, &clip, &ok), "timed out");
    assert_eq!(w.public(), Value::Null);
    let new = w.on(WATCH_MS).unwrap();
    assert!(!w.tick(old, WATCH_MS + 1, &mut seen, &clip, &ok), "an old watcher stops");
    assert!(w.tick(new, WATCH_MS + 1, &mut seen, &clip, &ok));
}

#[test]
fn the_route_turns_it_on_and_off() {
    let a = app(&temp_dir("watch"), 1, None);
    let state = |r: Option<Result<(u16, String, &'static str), String>>| -> Value {
        serde_json::from_str(&r.unwrap().unwrap().1).unwrap()
    };
    assert_eq!(serde_json::from_str::<Value>(&a.public_state()).unwrap()["geminiWatch"], Value::Null);
    let v = state(api(&a, "POST", "/api/gemini/watch", &json!({ "on": true })));
    assert_eq!(v["geminiWatch"]["state"], "waiting");
    assert!(v["geminiWatch"]["until"].as_i64().unwrap() > crate::now());
    assert!(api(&a, "POST", "/api/gemini/watch", &json!({})).unwrap().is_err(), "on must be given");
    let v = state(api(&a, "POST", "/api/gemini/watch", &json!({ "on": false })));
    assert_eq!(v["geminiWatch"], Value::Null);
}

#[test]
fn the_demo_does_not_watch() {
    let a = app(&temp_dir("watch-demo"), 1, Some(config()));
    let mut demo = std::sync::Arc::into_inner(a).unwrap();
    demo.demo = true;
    let a = std::sync::Arc::new(demo);
    let r = api(&a, "POST", "/api/gemini/watch", &json!({ "on": true })).unwrap();
    assert_eq!(r.unwrap_err(), "Not in the demo");
    assert_eq!(a.gemini_watch.public(), Value::Null);
}
