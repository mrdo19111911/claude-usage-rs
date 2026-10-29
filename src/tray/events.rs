// The tray's messages: hover, click, menu (menu.rs), refresh, exit.

use super::*;

// ---------- Messages ----------

pub(super) unsafe extern "system" fn msg_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if UI.with(|u| u.borrow().is_none()) {
        return DefWindowProcW(hwnd, msg, wp, lp);
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

pub(super) fn open_dashboard() {
    crate::open_browser(&ui(|u| u.url.clone()));
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
    // A fresh read: the model can be 2 s old.
    let state = ui(|u| u.state.clone());
    if exit_step(true, model(&state()).desk_switching) == ExitStep::Now {
        exit();
    }
    ui(|u| u.exit_pending = true);
    if from_menu {
        let text = wide("Claude Desktop is switching accounts. The dashboard will close as soon as it is done.");
        let style = MB_OK | MB_ICONINFORMATION | MB_TOPMOST | MB_SETFOREGROUND;
        unsafe { MessageBoxW(ptr::null_mut(), text.as_ptr(), wide("Claude usage").as_ptr(), style) };
    }
}

// Takes the icon out of the tray, then ends the program.
pub(super) fn exit() -> ! {
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

pub(super) unsafe fn refresh() {
    let state = ui(|u| u.state.clone());
    let m = model(&state());
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
