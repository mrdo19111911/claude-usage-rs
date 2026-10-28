// The tray's messages: hover, click, menu, refresh, exit.

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
        WM_EXIT_APP => exit(),
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

pub(super) unsafe fn menu(x: i32, y: i32) {
    let (msg, pin_on) = ui(|u| (u.msg, u.pin_on));
    let m = CreatePopupMenu();
    AppendMenuW(m, MF_STRING, ID_OPEN, wide("Open dashboard").as_ptr());
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
        ID_PIN if pin_on => unpin(),
        ID_PIN => pin(None),
        ID_QUIT => exit(),
        _ => {}
    }
}

// From any thread: the tray thread takes its icon out, then ends the program.
pub fn request_exit() {
    let h = MSG_HWND.load(Ordering::SeqCst);
    if h.is_null() {
        std::process::exit(0);
    }
    unsafe { PostMessageW(h, WM_EXIT_APP, 0, 0) };
}

// Takes the icon out of the tray, then ends the program.
pub(super) fn exit() -> ! {
    UI.with(|u| {
        if let Some(u) = u.borrow().as_ref() {
            let mut n: NOTIFYICONDATAW = unsafe { mem::zeroed() };
            n.cbSize = mem::size_of::<NOTIFYICONDATAW>() as u32;
            n.hWnd = u.msg;
            n.uID = 1;
            unsafe { Shell_NotifyIconW(NIM_DELETE, &n) };
        }
    });
    std::process::exit(0);
}

pub(super) unsafe fn refresh() {
    let state = ui(|u| u.state.clone());
    let m = model(&state());
    ui(|u| u.model = m);
    update_icon(false);
    for (w, visible) in ui(|u| [(u.hover, true), (u.pinned, u.pin_on)]) {
        if visible && IsWindowVisible(w) != 0 {
            fit(w);
            InvalidateRect(w, ptr::null(), FALSE);
        }
    }
}
