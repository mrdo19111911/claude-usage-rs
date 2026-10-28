// Where the hover widget goes, and when it hides.

use super::*;

// ---------- The widget ----------

pub(super) unsafe fn dpi_at(x: i32, y: i32) -> f32 {
    let mon = MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST);
    let (mut dx, mut dy) = (96, 96);
    GetDpiForMonitor(mon, MDT_EFFECTIVE_DPI, &mut dx, &mut dy);
    dx as f32 / 96.0
}

pub(super) unsafe fn work_area(x: i32, y: i32) -> RECT {
    let mon = MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST);
    let mut mi: MONITORINFO = mem::zeroed();
    mi.cbSize = mem::size_of::<MONITORINFO>() as u32;
    GetMonitorInfoW(mon, &mut mi);
    mi.rcWork
}

pub(super) fn width(s: f32) -> i32 {
    (360.0 * s) as i32
}

// Keeps the widget's top-left corner, sized to what it shows now.
pub(super) unsafe fn fit(w: HWND) {
    let s = GetDpiForWindow(w) as f32 / 96.0;
    let h = layout(&ui(|u| u.model.clone()), s, false, ptr::null_mut());
    SetWindowPos(w, HWND_TOPMOST, 0, 0, width(s), h, SWP_NOMOVE | SWP_NOACTIVATE);
}

// Above the icon (or beside it, for a taskbar at the side), inside the screen.
pub(super) unsafe fn show_hover(ax: i32, ay: i32) {
    let (w, m) = ui(|u| {
        u.away = 0;
        (u.hover, u.model.clone())
    });
    let s = dpi_at(ax, ay);
    let (ww, hh) = (width(s), layout(&m, s, false, ptr::null_mut()));
    let wa = work_area(ax, ay);
    let gap = (12.0 * s) as i32;
    let x = (ax - ww / 2).clamp(wa.left + gap, (wa.right - ww - gap).max(wa.left));
    let y = if ay >= wa.bottom { wa.bottom - hh - gap } else if ay < wa.top { wa.top + gap } else { (ay - hh - gap).max(wa.top + gap) };
    SetWindowPos(w, HWND_TOPMOST, x, y, ww, hh, SWP_NOACTIVATE | SWP_SHOWWINDOW);
    InvalidateRect(w, ptr::null(), FALSE);
    SetTimer(ui(|u| u.msg), TIMER_HOVER, 150, None);
}

pub(super) unsafe fn hide_hover() {
    let (msg, w) = ui(|u| (u.msg, u.hover));
    KillTimer(msg, TIMER_HOVER);
    ShowWindow(w, SW_HIDE);
}

pub(super) unsafe fn hover_check() {
    let (msg, w) = ui(|u| (u.msg, u.hover));
    let mut p = POINT { x: 0, y: 0 };
    GetCursorPos(&mut p);
    let inside = |r: &RECT| p.x >= r.left && p.x < r.right && p.y >= r.top && p.y < r.bottom;
    let mut r: RECT = mem::zeroed();
    GetWindowRect(w, &mut r);
    let mut id: NOTIFYICONIDENTIFIER = mem::zeroed();
    id.cbSize = mem::size_of::<NOTIFYICONIDENTIFIER>() as u32;
    id.hWnd = msg;
    id.uID = 1;
    let mut ir: RECT = mem::zeroed();
    let on_icon = Shell_NotifyIconGetRect(&id, &mut ir) == 0 && inside(&ir);
    // A few ticks of grace: the pointer crosses the gap between the icon and the widget.
    let away = ui(|u| {
        u.away = if inside(&r) || on_icon { 0 } else { u.away + 1 };
        u.away
    });
    if away >= 3 {
        hide_hover();
    }
}
