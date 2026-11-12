// The dashboard in its own window, with an embedded WebView2 (docs/specs/dashboard-window.md):
// a plain Win32 window made here, the WebView in it by wry, which follows its size itself.
// Where it opens: dash_rect.rs; which pages stay in it: dash_nav.rs.

#[path = "dash_nav.rs"]
mod nav;
#[path = "dash_rect.rs"]
mod rect;
#[path = "dash_spot.rs"]
mod spot;

use super::*;
use nav::stays_inside;
use spot::{keep_place, place};
use std::cell::Cell;
use std::num::NonZeroIsize;
use std::path::Path;
use windows_sys::Win32::Graphics::Dwm::DwmSetWindowAttribute;
use wry::raw_window_handle::{HandleError, HasWindowHandle, RawWindowHandle, Win32WindowHandle, WindowHandle};
use wry::{NewWindowResponse, Theme, WebContext, WebView, WebViewBuilder, WebViewBuilderExtWindows};

const CLASS: &str = "scas-dashboard";
const PLACE_FILE: &str = "dashboard_window.json";
// Windows 10 20H1+: a dark title bar (not in windows-sys under this name).
const DWMWA_USE_IMMERSIVE_DARK_MODE: u32 = 20;

thread_local! {
    // The open window (null when none), set as soon as it is made.
    static WIN: Cell<HWND> = const { Cell::new(ptr::null_mut()) };
    // Its WebView, the data folder's context, and where to keep its place.
    static VIEW: RefCell<Option<(WebView, WebContext, PathBuf)>> = const { RefCell::new(None) };
}

// Our window, as wry asks for one.
struct Parent(HWND);

impl HasWindowHandle for Parent {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let h = NonZeroIsize::new(self.0 as isize).ok_or(HandleError::Unavailable)?;
        Ok(unsafe { WindowHandle::borrow_raw(RawWindowHandle::Win32(Win32WindowHandle::new(h))) })
    }
}

// Brings the open window to the front, else makes it. False when it cannot be made (no
// WebView2 runtime): the caller opens the browser instead. `widget_file` is in the data folder.
pub(super) fn open(url: &str, widget_file: &Path) -> bool {
    let h = WIN.with(Cell::get);
    if !h.is_null() {
        unsafe {
            if IsIconic(h) != 0 {
                ShowWindow(h, SW_RESTORE);
            }
            SetForegroundWindow(h);
        }
        return true;
    }
    if wry::webview_version().is_err() {
        return false;
    }
    let dir = widget_file.parent().unwrap_or(Path::new("."));
    unsafe { make(url, dir) }
}

// The dashboard's port, from its URL ("http://127.0.0.1:4777").
fn port_of(url: &str) -> u16 {
    url.trim_end_matches('/').rsplit(':').next().and_then(|p| p.parse().ok()).unwrap_or(0)
}

unsafe fn make(url: &str, dir: &Path) -> bool {
    let inst = GetModuleHandleW(ptr::null());
    let class = wide(CLASS);
    static REGISTER: std::sync::Once = std::sync::Once::new();
    REGISTER.call_once(|| {
        let mut wc: WNDCLASSW = mem::zeroed();
        wc.hInstance = inst;
        wc.lpfnWndProc = Some(dash_proc);
        wc.lpszClassName = class.as_ptr();
        wc.hCursor = LoadCursorW(ptr::null_mut(), IDC_ARROW);
        wc.hIcon = LoadIconW(inst, 1 as _); // the exe's icon (build.rs)
        wc.hbrBackground = CreateSolidBrush(BG); // no white flash before the page
        RegisterClassW(&wc);
    });
    let file = dir.join(PLACE_FILE);
    let r = place(&file);
    let title = wide(words::APP);
    let h = CreateWindowExW(0, class.as_ptr(), title.as_ptr(), WS_OVERLAPPEDWINDOW, r.x, r.y, r.w, r.h, ptr::null_mut(), ptr::null_mut(), inst, ptr::null());
    if h.is_null() {
        return false;
    }
    let dark: i32 = 1;
    DwmSetWindowAttribute(h, DWMWA_USE_IMMERSIVE_DARK_MODE as _, &dark as *const i32 as _, 4);
    WIN.with(|w| w.set(h));
    // Not ShowWindow: its first call in a process takes the launcher's show mode (seen:
    // maximised when started from a shell).
    SetWindowPos(h, ptr::null_mut(), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_SHOWWINDOW);
    SetForegroundWindow(h);
    // Building pumps messages (wry waits for WebView2): no borrow is held across it.
    let mut ctx = WebContext::new(Some(dir.join("webview")));
    let port = port_of(url);
    let view = WebViewBuilder::new_with_web_context(&mut ctx)
        .with_url(url)
        .with_background_color((0x1a, 0x1a, 0x1b, 0xff))
        .with_theme(Theme::Dark)
        .with_navigation_handler(move |u| stays_inside(&u, port) || leave(&u))
        .with_new_window_req_handler(|u, _| {
            leave(&u);
            NewWindowResponse::Deny
        })
        .build(&Parent(h));
    match view {
        Ok(v) if WIN.with(Cell::get) == h => {
            VIEW.with(|s| *s.borrow_mut() = Some((v, ctx, file)));
            true
        }
        // Closed while it was being built: nothing to keep.
        Ok(_) => true,
        Err(e) => {
            eprintln!("Dashboard window: {e}; opening the browser instead.");
            DestroyWindow(h);
            false
        }
    }
}

// A page outside the dashboard: in the browser, not here.
fn leave(url: &str) -> bool {
    if url.starts_with("http://") || url.starts_with("https://") {
        crate::open_browser(url);
    }
    false
}

unsafe extern "system" fn dash_proc(h: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_GETMINMAXINFO => {
            let mi = &mut *(lp as *mut MINMAXINFO);
            mi.ptMinTrackSize = POINT { x: 480, y: 360 };
            0
        }
        WM_DPICHANGED => {
            let r = &*(lp as *const RECT);
            SetWindowPos(h, ptr::null_mut(), r.left, r.top, r.right - r.left, r.bottom - r.top, SWP_NOZORDER | SWP_NOACTIVATE);
            0
        }
        WM_CLOSE => {
            // The WebView first (its processes end), then the window.
            if let Some((view, ctx, file)) = VIEW.with(|s| s.borrow_mut().take()) {
                keep_place(h, &file);
                drop(view);
                drop(ctx);
            }
            DestroyWindow(h);
            0
        }
        WM_DESTROY => {
            WIN.with(|w| w.set(ptr::null_mut()));
            0
        }
        _ => DefWindowProcW(h, msg, wp, lp),
    }
}
