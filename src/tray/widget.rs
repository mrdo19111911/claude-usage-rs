// The widget window: pinning, clicks, dragging, painting.

use super::*;

pub(super) unsafe fn save_pin() {
    let (file, on, pos) = ui(|u| (u.file.clone(), u.pin_on, u.pin_pos));
    let mut v = json!({ "pinned": on });
    if let Some((x, y)) = pos {
        v["x"] = json!(x);
        v["y"] = json!(y);
    }
    if let Some(d) = file.parent() {
        let _ = fs::create_dir_all(d);
    }
    let _ = fs::write(file, v.to_string());
}

// Pins the widget: at `at` (where the hover widget was), else where it was last,
// else the top-right corner of the main screen.
pub(super) unsafe fn pin(at: Option<(i32, i32)>) {
    ui(|u| {
        u.pin_on = true;
        if at.is_some() {
            u.pin_pos = at;
        }
    });
    save_pin();
    show_pinned();
}

pub(super) unsafe fn unpin() {
    let w = ui(|u| {
        u.pin_on = false;
        u.pinned
    });
    ShowWindow(w, SW_HIDE);
    save_pin();
}

pub(super) unsafe fn show_pinned() {
    let (w, pos, m) = ui(|u| (u.pinned, u.pin_pos, u.model.clone()));
    // A saved place on a screen that is no longer there falls back to the corner.
    let pos = pos.filter(|&(x, y)| !MonitorFromPoint(POINT { x: x + 20, y: y + 20 }, MONITOR_DEFAULTTONULL).is_null());
    let (x, y) = pos.unwrap_or_else(|| {
        let wa = work_area(0, 0);
        let s = dpi_at(0, 0);
        (wa.right - width(s) - (16.0 * s) as i32, wa.top + (16.0 * s) as i32)
    });
    let place = |s: f32| {
        let (ww, hh) = (width(s), layout(&m, s, true, ptr::null_mut()));
        SetWindowPos(w, HWND_TOPMOST, x, y, ww, hh, SWP_NOACTIVATE | SWP_SHOWWINDOW);
    };
    // Twice: a move onto a screen with another DPI sends WM_DPICHANGED, whose handler
    // moves the window to Windows' suggested spot; the second call, at the window's
    // own DPI now, puts it back.
    place(dpi_at(x, y));
    place(GetDpiForWindow(w) as f32 / 96.0);
    InvalidateRect(w, ptr::null(), FALSE);
}

// The small button at the top right: "Pin" on the hover widget, "✕" on the pinned one.
pub(super) fn button(s: f32, w: i32) -> RECT {
    let p = (14.0 * s) as i32;
    RECT { left: w - p - (44.0 * s) as i32, top: (6.0 * s) as i32, right: w - p + (6.0 * s) as i32, bottom: (30.0 * s) as i32 }
}

pub(super) unsafe extern "system" fn widget_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if UI.with(|u| u.borrow().is_none()) {
        return DefWindowProcW(hwnd, msg, wp, lp);
    }
    let is_pinned = ui(|u| u.pinned == hwnd);
    let s = GetDpiForWindow(hwnd) as f32 / 96.0;
    let in_button = |x: i32, y: i32| {
        let mut rc: RECT = mem::zeroed();
        GetClientRect(hwnd, &mut rc);
        let b = button(s, rc.right);
        x >= b.left && x < b.right && y >= b.top && y < b.bottom
    };
    match msg {
        WM_PAINT => {
            paint(hwnd, is_pinned);
            0
        }
        WM_ERASEBKGND => 1,
        WM_MOUSEACTIVATE => MA_NOACTIVATE as LRESULT,
        // The pinned widget drags by any spot but its buttons.
        WM_NCHITTEST if is_pinned => {
            let mut p = POINT { x: (lp & 0xffff) as i16 as i32, y: (lp >> 16 & 0xffff) as i16 as i32 };
            ScreenToClient(hwnd, &mut p);
            if in_button(p.x, p.y) || hit_at(true, p.x, p.y).is_some() { HTCLIENT as LRESULT } else { HTCAPTION as LRESULT }
        }
        WM_LBUTTONUP => {
            let (x, y) = ((lp & 0xffff) as i16 as i32, (lp >> 16 & 0xffff) as i16 as i32);
            if in_button(x, y) {
                if is_pinned {
                    unpin();
                } else {
                    let mut r: RECT = mem::zeroed();
                    GetWindowRect(hwnd, &mut r);
                    hide_hover();
                    pin(Some((r.left, r.top)));
                }
            } else if let Some(k) = hit_at(is_pinned, x, y) {
                // A row's CLI or Desktop button.
                switch(hwnd, k);
            } else if !is_pinned {
                hide_hover();
                open_dashboard();
            }
            0
        }
        WM_NCLBUTTONDBLCLK if is_pinned => {
            open_dashboard();
            0
        }
        WM_EXITSIZEMOVE if is_pinned => {
            let mut r: RECT = mem::zeroed();
            GetWindowRect(hwnd, &mut r);
            ui(|u| u.pin_pos = Some((r.left, r.top)));
            save_pin();
            0
        }
        WM_DPICHANGED => {
            let r = &*(lp as *const RECT);
            let s = (wp >> 16 & 0xffff) as f32 / 96.0;
            let h = layout(&ui(|u| u.model.clone()), s, is_pinned, ptr::null_mut());
            SetWindowPos(hwnd, ptr::null_mut(), r.left, r.top, width(s), h, SWP_NOZORDER | SWP_NOACTIVATE);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

pub(super) unsafe fn paint(hwnd: HWND, pinned: bool) {
    let mut ps: PAINTSTRUCT = mem::zeroed();
    let dc = BeginPaint(hwnd, &mut ps);
    let mut rc: RECT = mem::zeroed();
    GetClientRect(hwnd, &mut rc);
    // Drawn off screen first: no flicker on the 2 s refresh.
    let mem_dc = CreateCompatibleDC(dc);
    let bmp = CreateCompatibleBitmap(dc, rc.right, rc.bottom);
    let old = SelectObject(mem_dc, bmp);
    let s = GetDpiForWindow(hwnd) as f32 / 96.0;
    layout(&ui(|u| u.model.clone()), s, pinned, mem_dc);
    BitBlt(dc, 0, 0, rc.right, rc.bottom, mem_dc, 0, 0, SRCCOPY);
    SelectObject(mem_dc, old);
    DeleteObject(bmp);
    DeleteDC(mem_dc);
    EndPaint(hwnd, &ps);
}
