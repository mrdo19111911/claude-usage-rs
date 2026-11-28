// Which running dashboard a window belongs to: the normal one and a --demo one can run
// side by side (their mutexes differ), so their tray windows must be told apart too.
// Also: one dashboard at a time, and `--quit` (the installer) ends the running one.

use super::*;
use windows_sys::Win32::System::Threading::{CreateMutexW, OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE};

// The tray message window's class for this instance; "-demo" for the demo one.
pub(super) fn tray_class(demo: bool) -> &'static str {
    if demo { "cud-tray-demo" } else { "cud-tray" }
}

// Whether the dashboard state (state_json) is the demo one's: its top-level "demo".
pub(super) fn is_demo(state: &str) -> bool {
    serde_json::from_str::<Value>(state).is_ok_and(|v| v["demo"].as_bool() == Some(true))
}

// From a second launch: asks the running tray of the same kind to open its dashboard.
// False if none found, or if it cannot be asked (Windows refuses a message to an elevated one).
pub fn signal_running(demo: bool) -> bool {
    let h = unsafe { FindWindowW(wide(tray_class(demo)).as_ptr(), ptr::null()) };
    !h.is_null() && unsafe { PostMessageW(h, WM_OPEN_APP, 0, 0) } != 0
}

// At start: false when this launch must end now. With `--quit` it asks the running one
// to exit, then ends with 0 once none runs (1 if it still does). Otherwise a named mutex
// (a random port no longer tells): a second launch asks the running one to open its
// dashboard instead.
pub fn claim_instance(demo: bool, quit: bool) -> bool {
    if quit {
        std::process::exit(if quit_class(tray_class(demo)) { 0 } else { 1 });
    }
    let name = wide(&format!("Local\\claude-usage-dashboard{}", if demo { "-demo" } else { "" }));
    // The handle lasts as long as this process.
    let h = unsafe { CreateMutexW(ptr::null(), 0, name.as_ptr()) };
    if first_copy(h.is_null(), unsafe { GetLastError() }) {
        return true;
    }
    eprintln!("The dashboard is already running: hover its icon in the tray.");
    // A double-click shows no stderr: the running one opens its dashboard instead. Its
    // window comes a moment after its mutex, so look for 3 s (docs/specs/relaunch.md).
    if !found_within(30, std::time::Duration::from_millis(100), || signal_running(demo)) {
        // Running with no window: nothing to signal, and silence would look like a dead app.
        let style = MB_OK | MB_ICONWARNING | MB_TOPMOST | MB_SETFOREGROUND;
        unsafe { MessageBoxW(ptr::null_mut(), wide(words::STUCK).as_ptr(), wide(words::APP).as_ptr(), style) };
    }
    false
}

// Whether this launch is the only copy, from CreateMutexW's answer: it made the mutex.
// No handle (`none`: the running copy's mutex refuses us, as when that one is elevated)
// is not the first copy either.
pub(super) fn first_copy(none: bool, last_error: u32) -> bool {
    !none && last_error != ERROR_ALREADY_EXISTS
}

// Calls `find` up to `tries` times, `gap` apart, until it says true.
pub(super) fn found_within(tries: u32, gap: std::time::Duration, mut find: impl FnMut() -> bool) -> bool {
    for i in 0..tries {
        if i > 0 {
            thread::sleep(gap);
        }
        if find() {
            return true;
        }
    }
    false
}

// Asks the tray window of this class to exit, as its Exit menu does (a Desktop switch under
// way finishes first), and waits up to a minute for its process to end. True once none runs.
pub(super) fn quit_class(class: &str) -> bool {
    let h = unsafe { FindWindowW(wide(class).as_ptr(), ptr::null()) };
    if h.is_null() {
        return true;
    }
    let mut pid = 0;
    unsafe { GetWindowThreadProcessId(h, &mut pid) };
    let p = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, pid) };
    unsafe { PostMessageW(h, WM_EXIT_APP, 0, 0) };
    if p.is_null() {
        return false;
    }
    let done = unsafe { WaitForSingleObject(p, 60_000) } == WAIT_OBJECT_0;
    unsafe { CloseHandle(p) };
    done
}
