// Which running dashboard a window belongs to: the normal one and a --demo one can run
// side by side (their mutexes differ), so their tray windows must be told apart too.

use super::*;

#[cfg(test)]
#[path = "instance_tests.rs"]
mod tests;

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
