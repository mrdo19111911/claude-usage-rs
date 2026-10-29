// The icon's right-click menu: the version, Open dashboard, Help, Check for updates, Pin
// widget, Exit. "Check for updates" asks GitHub off the tray thread and answers in a box
// (docs/specs/update.md, "Where it shows").

use super::*;
use std::sync::atomic::AtomicBool;

pub(super) const VERSION: &str = env!("CARGO_PKG_VERSION");
const TITLE: &str = "Claude Usage";
const COULD_NOT: &str = "Could not check for updates.";

// What the "Check for updates" box says: a plain note, a Yes/No offer to install, or a
// failure.
#[derive(Debug, PartialEq)]
pub(super) enum Answer {
    Info(String),
    Offer(String),
    Fail(String),
}

// The menu's first, greyed line.
pub(super) fn version_line() -> String {
    format!("Claude Usage {VERSION}")
}

// help.html, served next to the dashboard's index.html.
pub(super) fn help_url(url: &str) -> String {
    format!("{}/help.html", url.trim_end_matches('/'))
}

// The answer from the state JSON's "update" ({status, latest, url, checkedAt, error}),
// read after a check; `version` is this app's.
pub(super) fn update_answer(state: &str, version: &str) -> Answer {
    let v: Value = serde_json::from_str(state).unwrap_or_default();
    let u = &v["update"];
    match (u["status"].as_str(), u["latest"].as_str(), u["error"].as_str()) {
        (Some("upToDate"), _, _) => Answer::Info(format!("Claude Usage {version} is the latest version.")),
        (Some("available"), Some(latest), _) => Answer::Offer(format!(
            "Claude Usage {latest} is available (you have {version}). Install now? The app closes and opens again."
        )),
        (Some("error"), _, Some(e)) if !e.is_empty() => Answer::Fail(e.to_string()),
        _ => Answer::Fail(COULD_NOT.into()),
    }
}

pub(super) unsafe fn menu(x: i32, y: i32) {
    let (msg, pin_on) = ui(|u| (u.msg, u.pin_on));
    let m = CreatePopupMenu();
    AppendMenuW(m, MF_STRING | MF_GRAYED, ID_VERSION, wide(&version_line()).as_ptr());
    AppendMenuW(m, MF_SEPARATOR, 0, ptr::null());
    AppendMenuW(m, MF_STRING, ID_OPEN, wide("Open dashboard").as_ptr());
    AppendMenuW(m, MF_STRING, ID_HELP, wide("Help").as_ptr());
    let busy = if CHECKING.load(Ordering::SeqCst) { MF_GRAYED } else { 0 };
    AppendMenuW(m, MF_STRING | busy, ID_UPDATE, wide("Check for updates").as_ptr());
    AppendMenuW(m, MF_SEPARATOR, 0, ptr::null());
    AppendMenuW(m, MF_STRING | if pin_on { MF_CHECKED } else { 0 }, ID_PIN, wide("Pin widget on the desktop").as_ptr());
    AppendMenuW(m, MF_SEPARATOR, 0, ptr::null());
    AppendMenuW(m, MF_STRING, ID_QUIT, wide("Exit").as_ptr());
    // Without this the menu stays open after a click elsewhere.
    SetForegroundWindow(msg);
    let cmd = TrackPopupMenu(m, TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_NONOTIFY, x, y, 0, msg, ptr::null()) as usize;
    PostMessageW(msg, WM_NULL, 0, 0);
    DestroyMenu(m);
    match cmd {
        ID_OPEN => open_dashboard(),
        ID_HELP => crate::open_browser(&help_url(&ui(|u| u.url.clone()))),
        ID_UPDATE => check_updates(),
        ID_PIN if pin_on => unpin(),
        ID_PIN => pin(None),
        ID_QUIT => ask_exit(true),
        _ => {}
    }
}

// One check (and its install) at a time.
static CHECKING: AtomicBool = AtomicBool::new(false);

// Asks GitHub off the tray thread (it can take seconds), then answers in a box; Yes on
// an offer installs (the setup then closes this app).
fn check_updates() {
    if CHECKING.swap(true, Ordering::SeqCst) {
        return;
    }
    let (act, state) = ui(|u| (u.act.clone(), u.state.clone()));
    thread::spawn(move || {
        let answer = match act(Act::CheckUpdate) {
            Err(e) => Answer::Fail(e),
            Ok(()) => update_answer(&state(), VERSION),
        };
        match answer {
            Answer::Info(t) => {
                say(&t, MB_OK | MB_ICONINFORMATION);
            }
            Answer::Fail(t) => {
                say(&t, MB_OK | MB_ICONERROR);
            }
            Answer::Offer(t) => {
                if say(&t, MB_YESNO | MB_ICONQUESTION) == IDYES {
                    if let Err(e) = act(Act::InstallUpdate) {
                        say(&e, MB_OK | MB_ICONERROR);
                    }
                }
            }
        }
        CHECKING.store(false, Ordering::SeqCst);
    });
}

// A box on top of everything, no owner, like the tray's other boxes.
fn say(text: &str, style: MESSAGEBOX_STYLE) -> MESSAGEBOX_RESULT {
    let style = style | MB_TOPMOST | MB_SETFOREGROUND;
    unsafe { MessageBoxW(ptr::null_mut(), wide(text).as_ptr(), wide(TITLE).as_ptr(), style) }
}
