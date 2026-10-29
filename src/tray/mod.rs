// The tray icon, the way into the dashboard on Windows (no console, no browser tab at
// start). Hovering the icon shows a small widget with the figures: Claude CLI on top,
// Claude Desktop under it, then the other accounts. A click opens the dashboard, a
// right-click a menu. The widget can be pinned on the desktop: it then stays on top,
// drags anywhere and keeps its place (widget.json).
// Plain Win32 through windows-sys, drawn with GDI: SO_SANH_KHAY_WIDGET.md says why.
// One concern per file (docs/specs/modules.md); each starts with `use super::*;`.

mod buttons;
mod events;
mod icon;
mod instance;
mod layout;
mod menu;
mod model;
mod paint;
mod place;
mod sections;
mod widget;
pub use events::request_exit;
pub use instance::claim_instance;
use buttons::*;
use events::*;
use icon::*;
use instance::*;
use layout::*;
use menu::*;
use model::*;
use paint::*;
use place::*;
use sections::*;
use widget::*;
use serde_json::{json, Value};
use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::Arc;
use std::{fs, mem, ptr, thread};
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Time::{FileTimeToSystemTime, SystemTimeToTzSpecificLocalTime};
use windows_sys::Win32::UI::HiDpi::*;
use windows_sys::Win32::UI::Shell::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

// The dashboard's state as /api/state serves it.
pub type StateFn = Arc<dyn Fn() -> String + Send + Sync>;
// What a widget button asks for: Claude CLI on an account (its id), or Claude Desktop on
// an account's saved sign-in (its uuid). Blocking: call it off the tray thread.
pub enum Act {
    Cli(String),
    Desktop(String),
    // Ask GitHub for a newer version (the answer is in the state JSON's "update").
    CheckUpdate,
    // Download, verify and run the newer setup; the setup then closes this app.
    InstallUpdate,
}
pub type ActFn = Arc<dyn Fn(Act) -> Result<(), String> + Send + Sync>;

const WM_TRAY: u32 = WM_APP + 1;
// Exit asked from another thread (the dashboard's Exit button): see request_exit.
const WM_EXIT_APP: u32 = WM_APP + 2;
// A second launch asks this one to open the dashboard: see signal_running.
const WM_OPEN_APP: u32 = WM_APP + 3;
static MSG_HWND: AtomicPtr<core::ffi::c_void> = AtomicPtr::new(ptr::null_mut());
// The icon and the open widgets, every 2 s.
const TIMER_REFRESH: usize = 1;
// Hides the hover widget once the pointer has left both it and the icon.
const TIMER_HOVER: usize = 2;
// Enter or space on the focused icon (not in windows-sys).
const NIN_KEYSELECT: u32 = NIN_SELECT | 1;
const ID_OPEN: usize = 1;
const ID_PIN: usize = 2;
const ID_QUIT: usize = 3;
const ID_VERSION: usize = 4;
const ID_HELP: usize = 5;
const ID_UPDATE: usize = 6;

// Same palette as the page.
const BG: u32 = rgb(0x1a, 0x1a, 0x1b);
const BORDER: u32 = rgb(0x3a, 0x3a, 0x3c);
const TEXT: u32 = rgb(0xec, 0xec, 0xec);
const MUTED: u32 = rgb(0x9a, 0x9a, 0x9e);
const TRACK: u32 = rgb(0x2c, 0x2c, 0x2e);
const CODE: u32 = rgb(0x2b, 0xb5, 0xa8);
const DESK: u32 = rgb(0xa3, 0x7c, 0xf0);
const GREEN: u32 = rgb(0x3f, 0xb3, 0x4f);
const AMBER: u32 = rgb(0xf2, 0xb6, 0x32);
const RED: u32 = rgb(0xe5, 0x48, 0x4d);

const fn rgb(r: u32, g: u32, b: u32) -> u32 {
    r | g << 8 | b << 16
}

fn level(u: i64) -> u32 {
    if u >= 97 { RED } else if u >= 70 { AMBER } else { GREEN }
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

// ---------- The tray thread ----------

struct Ui {
    state: StateFn,
    act: ActFn,
    url: String,
    file: PathBuf,
    msg: HWND,
    hover: HWND,
    pinned: HWND,
    pin_on: bool,
    pin_pos: Option<(i32, i32)>,
    icon: HICON,
    icon_key: String,
    away: u32,
    taskbar_created: u32,
    model: Model,
    // Exit was asked while Claude Desktop was switching: exit once the switch is done.
    exit_pending: bool,
}

thread_local! {
    static UI: RefCell<Option<Ui>> = const { RefCell::new(None) };
}

// Each call takes what it needs and lets go: Win32 calls made after it can re-enter the
// window procedures, which read UI again.
fn ui<R>(f: impl FnOnce(&mut Ui) -> R) -> R {
    UI.with(|u| f(u.borrow_mut().as_mut().expect("tray not started")))
}

pub fn start(state: StateFn, act: ActFn, url: String, data_dir: PathBuf) {
    thread::spawn(move || unsafe { run(state, act, url, data_dir.join("widget.json")) });
}

unsafe fn run(state: StateFn, act: ActFn, url: String, file: PathBuf) {
    // Per thread: covers every window made here, unlike the process-wide call.
    SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    let inst = GetModuleHandleW(ptr::null());
    // Its own class, so a second launch signals the same kind (normal or demo).
    let msg_class = wide(tray_class(is_demo(&state())));
    let widget_class = wide("cud-widget");
    let mut wc: WNDCLASSW = mem::zeroed();
    wc.hInstance = inst;
    wc.lpfnWndProc = Some(msg_proc);
    wc.lpszClassName = msg_class.as_ptr();
    RegisterClassW(&wc);
    wc.lpfnWndProc = Some(widget_proc);
    wc.lpszClassName = widget_class.as_ptr();
    wc.hCursor = LoadCursorW(ptr::null_mut(), IDC_ARROW);
    wc.style = CS_DBLCLKS;
    RegisterClassW(&wc);

    let title = wide("Claude usage");
    // A hidden top-level window, not a message-only one: those miss TaskbarCreated.
    let msg = CreateWindowExW(0, msg_class.as_ptr(), title.as_ptr(), WS_POPUP, 0, 0, 0, 0, ptr::null_mut(), ptr::null_mut(), inst, ptr::null());
    let widget = || CreateWindowExW(WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE, widget_class.as_ptr(), title.as_ptr(), WS_POPUP, 0, 0, 0, 0, ptr::null_mut(), ptr::null_mut(), inst, ptr::null());
    let (hover, pinned) = (widget(), widget());

    let saved: Value = fs::read_to_string(&file).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
    let pin_pos = saved["x"].as_i64().zip(saved["y"].as_i64()).map(|(x, y)| (x as i32, y as i32));
    let model = model(&state());
    UI.with(|u| {
        *u.borrow_mut() = Some(Ui {
            state, act, url, file, msg, hover, pinned,
            pin_on: saved["pinned"].as_bool() == Some(true),
            pin_pos,
            icon: ptr::null_mut(),
            icon_key: String::new(),
            away: 0,
            taskbar_created: RegisterWindowMessageW(wide("TaskbarCreated").as_ptr()),
            model,
            exit_pending: false,
        })
    });
    add_icon();
    MSG_HWND.store(msg, Ordering::SeqCst);
    SetTimer(msg, TIMER_REFRESH, 2000, None);
    if ui(|u| u.pin_on) {
        show_pinned();
    }
    let mut m: MSG = mem::zeroed();
    while GetMessageW(&mut m, ptr::null_mut(), 0, 0) > 0 {
        TranslateMessage(&m);
        DispatchMessageW(&m);
    }
}
