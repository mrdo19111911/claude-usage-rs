// The dashboard window's place on the screen: where it opens, and keeping it for next
// time (docs/specs/dashboard-window.md). The rule itself: dash_rect.rs.

use super::super::*;
use super::rect::*;
use std::path::Path;

// Where it opens: its saved place, else centred on the screen under the pointer.
pub(super) unsafe fn place(file: &Path) -> Rect {
    let saved = fs::read_to_string(file).ok().and_then(|s| serde_json::from_str::<Value>(&s).ok()).and_then(|v| rect_from_json(&v));
    let mon = match saved {
        Some(s) => MonitorFromRect(&RECT { left: s.x, top: s.y, right: s.x + s.w, bottom: s.y + s.h }, MONITOR_DEFAULTTONEAREST),
        None => {
            let mut pt = POINT { x: 0, y: 0 };
            GetCursorPos(&mut pt);
            MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST)
        }
    };
    let mut mi: MONITORINFO = mem::zeroed();
    mi.cbSize = mem::size_of::<MONITORINFO>() as u32;
    GetMonitorInfoW(mon, &mut mi);
    let w = mi.rcWork;
    let (mut dx, mut dy) = (96u32, 96u32);
    GetDpiForMonitor(mon, MDT_EFFECTIVE_DPI, &mut dx, &mut dy);
    first_rect(saved, Rect { x: w.left, y: w.top, w: w.right - w.left, h: w.bottom - w.top }, dx as f32 / 96.0)
}

// Keeps its place (not maximised or minimised) for next time.
pub(super) unsafe fn keep_place(h: HWND, file: &Path) {
    if IsIconic(h) != 0 || IsZoomed(h) != 0 {
        return;
    }
    let mut r: RECT = mem::zeroed();
    GetWindowRect(h, &mut r);
    let rect = Rect { x: r.left, y: r.top, w: r.right - r.left, h: r.bottom - r.top };
    let _ = fs::write(file, rect_json(rect).to_string());
}
