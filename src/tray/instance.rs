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
// False if none found.
pub fn signal_running(demo: bool) -> bool {
    let h = unsafe { FindWindowW(wide(tray_class(demo)).as_ptr(), ptr::null()) };
    if h.is_null() {
        return false;
    }
    unsafe { PostMessageW(h, WM_OPEN_APP, 0, 0) };
    true
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
    unsafe { CreateMutexW(ptr::null(), 0, name.as_ptr()) };
    if unsafe { GetLastError() } != ERROR_ALREADY_EXISTS {
        return true;
    }
    eprintln!("The dashboard is already running: hover its icon in the tray.");
    // A double-click shows no stderr: the running one opens its dashboard instead.
    signal_running(demo);
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
