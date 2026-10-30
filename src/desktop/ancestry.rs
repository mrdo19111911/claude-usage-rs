// Whether this process descends from one of a set (Claude's), by parent process ids.
// Windows gives the id of a process that ended to a new one: a "parent" that started
// after its child is a stranger with a reused id, and the line ends there. Seen
// 2026-10-30: the dashboard, started through a short-lived explorer.exe, looked like a
// child of a conhost.exe of Claude's that started 8 minutes after it.

use std::collections::HashMap;

// `parent`: each running process's parent id; `started`: when a process started (None
// when Windows does not say: then the ids are followed as they are).
pub(crate) fn descends_from(parent: &HashMap<u32, u32>, started: impl Fn(u32) -> Option<u64>, me: u32, set: &[u32]) -> bool {
    let mut pid = me;
    for _ in 0..64 {
        match parent.get(&pid) {
            Some(&p) if p != 0 && p != pid => {
                if let (Some(child), Some(par)) = (started(pid), started(p)) {
                    if par > child {
                        return false;
                    }
                }
                if set.contains(&p) {
                    return true;
                }
                pid = p;
            }
            _ => break,
        }
    }
    false
}

// When a process started (FILETIME, 100 ns since 1601); None when it cannot be opened.
#[cfg(windows)]
pub(crate) fn started_at(pid: u32) -> Option<u64> {
    use windows_sys::Win32::Foundation::{CloseHandle, FILETIME};
    use windows_sys::Win32::System::Threading::{GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if h.is_null() {
            return None;
        }
        let zero = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
        let (mut c, mut e, mut k, mut u) = (zero, zero, zero, zero);
        let ok = GetProcessTimes(h, &mut c, &mut e, &mut k, &mut u) != 0;
        CloseHandle(h);
        ok.then(|| (c.dwHighDateTime as u64) << 32 | c.dwLowDateTime as u64)
    }
}
