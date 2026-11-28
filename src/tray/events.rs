// The tray's messages: hover, click, menu (menu.rs), refresh, exit.

use super::*;

// ---------- Messages ----------

pub(super) unsafe extern "system" fn msg_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if UI.with(|u| u.borrow().is_none()) {
        return DefWindowProcW(hwnd, msg, wp, lp);
    }
    match closing(msg, wp) {
        Some(Closing::Ask) => {
            ask_exit(false);
            return 0;
        }
        Some(Closing::Now) => exit(),
        None => {}
    }
    match msg {
        WM_TRAY => {
            // NOTIFYICON_VERSION_4: the event in lParam's low word, the icon's place in wParam.
            let (x, y) = ((wp & 0xffff) as i16 as i32, (wp >> 16 & 0xffff) as i16 as i32);
            match (lp & 0xffff) as u32 {
                NIN_POPUPOPEN => show_hover(x, y),
                NIN_SELECT | NIN_KEYSELECT => {
                    hide_hover();
                    open_dashboard();
                }
                WM_CONTEXTMENU => {
                    hide_hover();
                    menu(x, y);
                }
                _ => {}
            }
            0
        }
        WM_EXIT_APP => {
            ask_exit(false);
            0
        }
        WM_OPEN_APP => {
            open_dashboard();
            0
        }
        WM_TIMER if wp == TIMER_REFRESH => {
            refresh();
            0
        }
        WM_TIMER if wp == TIMER_HOVER => {
            hover_check();
            0
        }
        _ if msg == ui(|u| u.taskbar_created) => {
            // Explorer restarted: the tray is new and empty.
            add_icon();
            0
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

// What a message asking the tray window to go means for the program. It must end with its
// window: left running it would hold the single-instance mutex with no icon, and every
// later launch would exit unseen (docs/specs/relaunch.md).
#[derive(Debug, PartialEq)]
pub(super) enum Closing {
    // As the Exit menu: a Desktop switch under way finishes first.
    Ask,
    Now,
}

pub(super) fn closing(msg: u32, wp: WPARAM) -> Option<Closing> {
    match msg {
        WM_CLOSE => Some(Closing::Ask),
        WM_ENDSESSION if wp != 0 => Some(Closing::Now),
        WM_DESTROY => Some(Closing::Now),
        _ => None,
    }
}

// In its own window (dash.rs); in the browser when that cannot be made (no WebView2).
pub(super) fn open_dashboard() {
    let (url, file) = ui(|u| (u.url.clone(), u.file.clone()));
    if !dash::open(&url, &file) {
        crate::open_browser(&url);
    }
}

// From main at start (--open): once the tray's window is up, it opens the dashboard.
pub fn request_open() {
    thread::spawn(|| {
        for _ in 0..100 {
            let h = MSG_HWND.load(Ordering::SeqCst);
            if !h.is_null() {
                unsafe { PostMessageW(h, WM_OPEN_APP, 0, 0) };
                return;
            }
            thread::sleep(std::time::Duration::from_millis(50));
        }
    });
}

// From any thread: the tray thread takes its icon out, then ends the program.
pub fn request_exit() {
    let h = MSG_HWND.load(Ordering::SeqCst);
    if h.is_null() {
        std::process::exit(0);
    }
    unsafe { PostMessageW(h, WM_EXIT_APP, 0, 0) };
}

// What an exit request does now: a Desktop switch killed midway skips its rollback.
#[derive(Debug, PartialEq)]
pub(super) enum ExitStep {
    Now,
    Wait,
    Stay,
}

pub(super) fn exit_step(asked: bool, switching: bool) -> ExitStep {
    match (asked, switching) {
        (false, _) => ExitStep::Stay,
        (true, true) => ExitStep::Wait,
        (true, false) => ExitStep::Now,
    }
}

// Every exit request (menu, dashboard): at once, or once the Desktop switch is done.
pub(super) fn ask_exit(from_menu: bool) {
    // A fresh read: the model can be 30 s old.
    let state = ui(|u| u.state.clone());
    if exit_step(true, model(&state()).desk_switching) == ExitStep::Now {
        exit();
    }
    ui(|u| u.exit_pending = true);
    // The next refresh sees the switch and watches it closely (tick_ms).
    changed();
    if from_menu {
        let text = wide(words::EXIT_WAIT);
        let style = MB_OK | MB_ICONINFORMATION | MB_TOPMOST | MB_SETFOREGROUND;
        unsafe { MessageBoxW(ptr::null_mut(), text.as_ptr(), wide(words::APP).as_ptr(), style) };
    }
}

// Takes the icon out of the tray, then ends the program.
pub(super) fn exit() -> ! {
    dash::keep();
    // The window out of the borrow first: Shell_NotifyIconW can re-enter msg_proc.
    if let Some(msg) = UI.with(|u| u.borrow().as_ref().map(|u| u.msg)) {
        let mut n: NOTIFYICONDATAW = unsafe { mem::zeroed() };
        n.cbSize = mem::size_of::<NOTIFYICONDATAW>() as u32;
        n.hWnd = msg;
        n.uID = 1;
        unsafe { Shell_NotifyIconW(NIM_DELETE, &n) };
    }
    std::process::exit(0);
}

// How long until the tray reads the state on its own. Changes come at once (`changed`);
// this only moves the texts that follow the clock ("2 min ago", "resets in 2h05m"). A
// Claude Desktop switch ends with no word, so it is watched closely while it runs.
pub(super) fn tick_ms(desk_switching: bool) -> u32 {
    if desk_switching { 2_000 } else { 30_000 }
}

// From any thread, the state lock held or not (posted, never sent): the state changed,
// the tray reads it again. Nothing without a tray window (tests, before start).
pub fn changed() {
    let h = MSG_HWND.load(Ordering::SeqCst);
    if !h.is_null() {
        unsafe { PostMessageW(h, WM_TIMER, TIMER_REFRESH, 0) };
    }
}

pub(super) unsafe fn refresh() {
    let state = ui(|u| u.state.clone());
    let m = model(&state());
    SetTimer(ui(|u| u.msg), TIMER_REFRESH, tick_ms(m.desk_switching), None);
    // An exit asked during a Desktop switch: the switch is done now.
    if exit_step(ui(|u| u.exit_pending), m.desk_switching) == ExitStep::Now {
        exit();
    }
    ui(|u| u.model = m);
    update_icon(false);
    for (w, visible) in ui(|u| [(u.hover, true), (u.pinned, u.pin_on)]) {
        if visible && IsWindowVisible(w) != 0 {
            fit(w);
            InvalidateRect(w, ptr::null(), FALSE);
        }
    }
}
