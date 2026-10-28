// The tray icon, the way into the dashboard on Windows (no console, no browser tab at
// start). Hovering the icon shows a small widget with the figures: Claude CLI on top,
// Claude Desktop under it, then the other accounts. A click opens the dashboard, a
// right-click a menu. The widget can be pinned on the desktop: it then stays on top,
// drags anywhere and keeps its place (widget.json).
// Plain Win32 through windows-sys, drawn with GDI: SO_SANH_KHAY_WIDGET.md says why.

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

const WM_TRAY: u32 = WM_APP + 1;
// Exit asked from another thread (the dashboard's Exit button): see request_exit.
const WM_EXIT_APP: u32 = WM_APP + 2;
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

// ---------- What the widget shows ----------

#[derive(Clone, Default)]
struct Win {
    u: i64,
    reset: Option<i64>,
}

#[derive(Clone, Default)]
struct Use {
    five: Win,
    week: Win,
}

#[derive(Clone, Default)]
struct Side {
    email: Option<String>,
    usage: Option<Use>,
}

#[derive(Clone, Default)]
struct Model {
    now: i64,
    threshold: i64,
    cli: Side,
    // None: no Claude Desktop on this computer.
    desk: Option<Side>,
    others: Vec<(String, Option<Use>)>,
}

fn usage(v: &Value) -> Option<Use> {
    let win = |w: &Value| Win { u: w["u"].as_i64().unwrap_or(0), reset: w["resetsAt"].as_i64() };
    v.is_object().then(|| Use { five: win(&v["five"]), week: win(&v["week"]) })
}

fn model(state: &str) -> Model {
    let v: Value = serde_json::from_str(state).unwrap_or_default();
    let mut m = Model { now: v["now"].as_i64().unwrap_or(0), threshold: v["settings"]["threshold"].as_i64().unwrap_or(95), ..Default::default() };
    let active = v["activeId"].as_str();
    for a in v["accounts"].as_array().into_iter().flatten() {
        let email = a["email"].as_str().unwrap_or("?").to_string();
        if active.is_some() && a["id"].as_str() == active {
            m.cli = Side { email: Some(email), usage: usage(&a["usage"]) };
        } else {
            m.others.push((email, usage(&a["usage"])));
        }
    }
    let d = &v["desktop"];
    if d["found"].as_bool() == Some(true) && v["demo"].as_bool() != Some(true) {
        let on = d["accounts"].as_array().and_then(|l| l.iter().find(|a| a["signedIn"].as_bool() == Some(true)));
        m.desk = Some(on.map_or_else(Side::default, |a| Side {
            email: a["email"].as_str().or(a["label"].as_str()).map(String::from),
            usage: usage(&a["usage"]),
        }));
    }
    m
}

// A moment (ms since 1970) on this computer's clock: "22:20" today, else "Thu 11:00".
fn clock(t: i64, now: i64) -> String {
    let local = |ms: i64| unsafe {
        let ft = (ms.max(0) as u64 + 11_644_473_600_000) * 10_000;
        let ft = FILETIME { dwLowDateTime: ft as u32, dwHighDateTime: (ft >> 32) as u32 };
        let (mut utc, mut loc): (SYSTEMTIME, SYSTEMTIME) = (mem::zeroed(), mem::zeroed());
        FileTimeToSystemTime(&ft, &mut utc);
        SystemTimeToTzSpecificLocalTime(ptr::null(), &utc, &mut loc);
        loc
    };
    let (a, b) = (local(t), local(now));
    let hm = format!("{:02}:{:02}", a.wHour, a.wMinute);
    if (a.wYear, a.wMonth, a.wDay) == (b.wYear, b.wMonth, b.wDay) {
        hm
    } else {
        format!("{} {hm}", ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"][a.wDayOfWeek as usize % 7])
    }
}

// "4h39m", "2d 3h", "12m".
fn left(ms: i64) -> String {
    let m = (ms.max(0) + 59_999) / 60_000;
    if m >= 1440 { format!("{}d {}h", m / 1440, m % 1440 / 60) } else if m >= 60 { format!("{}h{:02}m", m / 60, m % 60) } else { format!("{m}m") }
}

// ---------- The tray thread ----------

struct Ui {
    state: StateFn,
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
}

thread_local! {
    static UI: RefCell<Option<Ui>> = const { RefCell::new(None) };
}

// Each call takes what it needs and lets go: Win32 calls made after it can re-enter the
// window procedures, which read UI again.
fn ui<R>(f: impl FnOnce(&mut Ui) -> R) -> R {
    UI.with(|u| f(u.borrow_mut().as_mut().expect("tray not started")))
}

pub fn start(state: StateFn, url: String, data_dir: PathBuf) {
    thread::spawn(move || unsafe { run(state, url, data_dir.join("widget.json")) });
}

unsafe fn run(state: StateFn, url: String, file: PathBuf) {
    SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    let inst = GetModuleHandleW(ptr::null());
    let msg_class = wide("cud-tray");
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
            state, url, file, msg, hover, pinned,
            pin_on: saved["pinned"].as_bool() == Some(true),
            pin_pos,
            icon: ptr::null_mut(),
            icon_key: String::new(),
            away: 0,
            taskbar_created: RegisterWindowMessageW(wide("TaskbarCreated").as_ptr()),
            model,
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

// ---------- The icon ----------

fn nid() -> NOTIFYICONDATAW {
    let mut n: NOTIFYICONDATAW = unsafe { mem::zeroed() };
    n.cbSize = mem::size_of::<NOTIFYICONDATAW>() as u32;
    n.hWnd = ui(|u| u.msg);
    n.uID = 1;
    n
}

fn tip(n: &mut NOTIFYICONDATAW, m: &Model) {
    let t = match (&m.cli.email, &m.cli.usage) {
        (Some(e), Some(u)) => format!("Claude CLI: {e}\n5-hour {}% · weekly {}%", u.five.u, u.week.u),
        (Some(e), None) => format!("Claude CLI: {e}"),
        _ => "Claude usage".into(),
    };
    for (d, s) in n.szTip.iter_mut().zip(t.encode_utf16().take(127).chain(Some(0))) {
        *d = s;
    }
}

unsafe fn add_icon() {
    update_icon(true);
}

// The icon draws the Claude CLI account's two limits as two bars, in the page's colors.
unsafe fn update_icon(add: bool) {
    let m = ui(|u| u.model.clone());
    let dpi = GetDpiForSystem();
    let size = GetSystemMetricsForDpi(SM_CXSMICON, dpi).max(16);
    let (five, week) = m.cli.usage.as_ref().map_or((None, None), |u| (Some(u.five.u), Some(u.week.u)));
    let key = format!("{size}/{five:?}/{week:?}");
    if !add && ui(|u| u.icon_key == key) {
        return;
    }
    let icon = make_icon(size, five, week);
    let mut n = nid();
    n.uFlags = NIF_ICON | NIF_TIP | NIF_MESSAGE;
    n.uCallbackMessage = WM_TRAY;
    n.hIcon = icon;
    tip(&mut n, &m);
    if add {
        Shell_NotifyIconW(NIM_ADD, &n);
        n.Anonymous.uVersion = NOTIFYICON_VERSION_4;
        Shell_NotifyIconW(NIM_SETVERSION, &n);
    } else {
        Shell_NotifyIconW(NIM_MODIFY, &n);
    }
    let old = ui(|u| {
        u.icon_key = key;
        mem::replace(&mut u.icon, icon)
    });
    if !old.is_null() {
        DestroyIcon(old);
    }
}

// Top bar: 5-hour limit; bottom bar: weekly. Gray while there are no figures.
unsafe fn make_icon(size: i32, five: Option<i64>, week: Option<i64>) -> HICON {
    let n = size as usize;
    let argb = |c: u32| 0xff00_0000 | (c & 0xff) << 16 | (c & 0xff00) | (c >> 16 & 0xff);
    let mut px = vec![0u32; n * n];
    let r = (n / 4) as f32;
    for y in 0..n {
        for x in 0..n {
            // Rounded square: outside a corner's quarter circle stays transparent.
            let (cx, cy) = ((x as f32 + 0.5).clamp(r, n as f32 - r), (y as f32 + 0.5).clamp(r, n as f32 - r));
            if (x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2) <= r * r {
                px[y * n + x] = argb(rgb(0x26, 0x26, 0x28));
            }
        }
    }
    let m = (n / 6).max(2);
    let gap = (n / 8).max(1);
    let bh = (n - 2 * m - gap) / 2;
    let inner = n - 2 * m;
    for (i, u) in [five, week].into_iter().enumerate() {
        let y0 = m + i * (bh + gap);
        let fill = u.map_or(0, |u| (inner as i64 * u.clamp(0, 100) as i64 + 99) / 100) as usize;
        let fill = if u.is_some_and(|u| u > 0) { fill.max(1) } else { fill };
        for y in y0..y0 + bh {
            for x in m..m + inner {
                let c = if x - m < fill { level(u.unwrap_or(0)) } else if u.is_some() { rgb(0x4a, 0x4a, 0x4e) } else { rgb(0x5a, 0x5a, 0x5e) };
                px[y * n + x] = argb(c);
            }
        }
    }
    let mut bmi: BITMAPINFO = mem::zeroed();
    bmi.bmiHeader.biSize = mem::size_of::<BITMAPINFOHEADER>() as u32;
    bmi.bmiHeader.biWidth = size;
    bmi.bmiHeader.biHeight = -size;
    bmi.bmiHeader.biPlanes = 1;
    bmi.bmiHeader.biBitCount = 32;
    bmi.bmiHeader.biCompression = BI_RGB;
    let mut bits: *mut core::ffi::c_void = ptr::null_mut();
    let color = CreateDIBSection(ptr::null_mut(), &bmi, DIB_RGB_COLORS, &mut bits, ptr::null_mut(), 0);
    ptr::copy_nonoverlapping(px.as_ptr(), bits as *mut u32, n * n);
    let mask = CreateBitmap(size, size, 1, 1, ptr::null());
    let info = ICONINFO { fIcon: TRUE, xHotspot: 0, yHotspot: 0, hbmMask: mask, hbmColor: color };
    let icon = CreateIconIndirect(&info);
    DeleteObject(color);
    DeleteObject(mask);
    icon
}

// ---------- Messages ----------

unsafe extern "system" fn msg_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
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

fn open_dashboard() {
    crate::open_browser(&ui(|u| u.url.clone()));
}

unsafe fn menu(x: i32, y: i32) {
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
fn exit() -> ! {
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

unsafe fn refresh() {
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

// ---------- The widget ----------

unsafe fn dpi_at(x: i32, y: i32) -> f32 {
    let mon = MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST);
    let (mut dx, mut dy) = (96, 96);
    GetDpiForMonitor(mon, MDT_EFFECTIVE_DPI, &mut dx, &mut dy);
    dx as f32 / 96.0
}

unsafe fn work_area(x: i32, y: i32) -> RECT {
    let mon = MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST);
    let mut mi: MONITORINFO = mem::zeroed();
    mi.cbSize = mem::size_of::<MONITORINFO>() as u32;
    GetMonitorInfoW(mon, &mut mi);
    mi.rcWork
}

fn width(s: f32) -> i32 {
    (360.0 * s) as i32
}

// Keeps the widget's top-left corner, sized to what it shows now.
unsafe fn fit(w: HWND) {
    let s = GetDpiForWindow(w) as f32 / 96.0;
    let h = layout(&ui(|u| u.model.clone()), s, false, ptr::null_mut());
    SetWindowPos(w, HWND_TOPMOST, 0, 0, width(s), h, SWP_NOMOVE | SWP_NOACTIVATE);
}

// Above the icon (or beside it, for a taskbar at the side), inside the screen.
unsafe fn show_hover(ax: i32, ay: i32) {
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

unsafe fn hide_hover() {
    let (msg, w) = ui(|u| (u.msg, u.hover));
    KillTimer(msg, TIMER_HOVER);
    ShowWindow(w, SW_HIDE);
}

unsafe fn hover_check() {
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

unsafe fn save_pin() {
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
unsafe fn pin(at: Option<(i32, i32)>) {
    ui(|u| {
        u.pin_on = true;
        if at.is_some() {
            u.pin_pos = at;
        }
    });
    save_pin();
    show_pinned();
}

unsafe fn unpin() {
    let w = ui(|u| {
        u.pin_on = false;
        u.pinned
    });
    ShowWindow(w, SW_HIDE);
    save_pin();
}

unsafe fn show_pinned() {
    let (w, pos, m) = ui(|u| (u.pinned, u.pin_pos, u.model.clone()));
    // A saved place on a screen that is no longer there falls back to the corner.
    let pos = pos.filter(|&(x, y)| !MonitorFromPoint(POINT { x: x + 20, y: y + 20 }, MONITOR_DEFAULTTONULL).is_null());
    let (x, y) = pos.unwrap_or_else(|| {
        let wa = work_area(0, 0);
        let s = dpi_at(0, 0);
        (wa.right - width(s) - (16.0 * s) as i32, wa.top + (16.0 * s) as i32)
    });
    let s = dpi_at(x, y);
    SetWindowPos(w, HWND_TOPMOST, x, y, width(s), layout(&m, s, true, ptr::null_mut()), SWP_NOACTIVATE | SWP_SHOWWINDOW);
    InvalidateRect(w, ptr::null(), FALSE);
}

// The small button at the top right: "Pin" on the hover widget, "✕" on the pinned one.
fn button(s: f32, w: i32) -> RECT {
    let p = (14.0 * s) as i32;
    RECT { left: w - p - (44.0 * s) as i32, top: (6.0 * s) as i32, right: w - p + (6.0 * s) as i32, bottom: (30.0 * s) as i32 }
}

unsafe extern "system" fn widget_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
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
        // The pinned widget drags by any spot but its button.
        WM_NCHITTEST if is_pinned => {
            let mut p = POINT { x: (lp & 0xffff) as i16 as i32, y: (lp >> 16 & 0xffff) as i16 as i32 };
            ScreenToClient(hwnd, &mut p);
            if in_button(p.x, p.y) { HTCLIENT as LRESULT } else { HTCAPTION as LRESULT }
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

unsafe fn paint(hwnd: HWND, pinned: bool) {
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

struct Painter {
    dc: HDC,
    s: f32,
    fonts: [HFONT; 3],
}

const NORMAL: usize = 0;
const BOLD: usize = 1;
const SMALL: usize = 2;

impl Painter {
    fn px(&self, v: f32) -> i32 {
        (v * self.s).round() as i32
    }
    unsafe fn fill(&self, l: i32, t: i32, r: i32, b: i32, c: u32) {
        let br = CreateSolidBrush(c);
        FillRect(self.dc, &RECT { left: l, top: t, right: r, bottom: b }, br);
        DeleteObject(br);
    }
    unsafe fn width_of(&self, s: &str, font: usize) -> i32 {
        SelectObject(self.dc, self.fonts[font]);
        let t: Vec<u16> = s.encode_utf16().collect();
        let mut sz = SIZE { cx: 0, cy: 0 };
        GetTextExtentPoint32W(self.dc, t.as_ptr(), t.len() as i32, &mut sz);
        sz.cx
    }
    // Cut with "…" to fit `max` pixels.
    unsafe fn fitted(&self, s: &str, font: usize, max: i32) -> String {
        if self.width_of(s, font) <= max {
            return s.into();
        }
        let mut c: Vec<char> = s.chars().collect();
        while !c.is_empty() && self.width_of(&format!("{}…", c.iter().collect::<String>()), font) > max {
            c.pop();
        }
        format!("{}…", c.iter().collect::<String>())
    }
    unsafe fn text(&self, x: i32, y: i32, s: &str, c: u32, font: usize) {
        SelectObject(self.dc, self.fonts[font]);
        SetTextColor(self.dc, c);
        let t: Vec<u16> = s.encode_utf16().collect();
        TextOutW(self.dc, x, y, t.as_ptr(), t.len() as i32);
    }
    unsafe fn text_right(&self, r: i32, y: i32, s: &str, c: u32, font: usize) {
        self.text(r - self.width_of(s, font), y, s, c, font);
    }
}

// Draws the widget when `dc` is set; returns its height either way.
unsafe fn layout(m: &Model, s: f32, pinned: bool, dc: HDC) -> i32 {
    let w = width(s);
    let font = |h: f32, weight: i32| {
        if dc.is_null() {
            return ptr::null_mut();
        }
        CreateFontW(-(h * s).round() as i32, 0, 0, 0, weight, 0, 0, 0, DEFAULT_CHARSET as u32, OUT_DEFAULT_PRECIS as u32, CLIP_DEFAULT_PRECIS as u32, CLEARTYPE_QUALITY as u32, DEFAULT_PITCH as u32, wide("Segoe UI").as_ptr())
    };
    let p = Painter { dc, s, fonts: [font(13.0, 400), font(13.0, 600), font(11.5, 400)] };
    let draw = !dc.is_null();
    let pad = p.px(14.0);
    let mut y = p.px(10.0);
    if draw {
        SetBkMode(dc, TRANSPARENT as i32);
        p.fill(0, 0, w, 10_000, BG);
        p.text(pad, y, "CLAUDE USAGE", MUTED, SMALL);
        let b = button(s, w);
        p.fill(b.left, b.top, b.right, b.bottom, rgb(0x2a, 0x2a, 0x2d));
        let label = if pinned { "✕" } else { "Pin" };
        p.text(b.left + (b.right - b.left - p.width_of(label, SMALL)) / 2, b.top + p.px(4.0), label, TEXT, SMALL);
    }
    y += p.px(28.0);

    let side = |y: &mut i32, title: &str, note: &str, color: u32, sd: &Side| {
        if draw {
            p.text(pad, *y, title, color, BOLD);
            p.text_right(w - pad, *y + p.px(1.5), note, MUTED, SMALL);
            let email = sd.email.as_deref().map_or("Not signed in".to_string(), |e| p.fitted(e, NORMAL, w - 2 * pad));
            p.text(pad, *y + p.px(19.0), &email, if sd.email.is_some() { TEXT } else { MUTED }, NORMAL);
        }
        *y += p.px(42.0);
        if let Some(u) = &sd.usage {
            for (label, win) in [("5h", &u.five), ("Week", &u.week)] {
                if draw {
                    let (bl, br) = (pad + p.px(42.0), w - pad - p.px(158.0));
                    let (bt, bb) = (*y + p.px(6.0), *y + p.px(13.0));
                    p.text(pad, *y, label, MUTED, SMALL);
                    p.fill(bl, bt, br, bb, TRACK);
                    let f = ((br - bl) as i64 * win.u.clamp(0, 100) / 100) as i32;
                    p.fill(bl, bt, bl + if win.u > 0 { f.max(p.px(3.0)) } else { 0 }, bb, level(win.u));
                    p.text_right(w - pad - p.px(114.0), *y - p.px(1.0), &format!("{}%", win.u), TEXT, BOLD);
                    let reset = win.reset.filter(|&t| t > m.now).map_or(String::new(), |t| format!("{} · {}", left(t - m.now), clock(t, m.now)));
                    p.text_right(w - pad, *y, &reset, MUTED, SMALL);
                }
                *y += p.px(20.0);
            }
        }
        *y += p.px(8.0);
    };
    side(&mut y, "Claude CLI", "terminal · VS Code", CODE, &m.cli);
    if let Some(d) = &m.desk {
        if draw {
            p.fill(pad, y - p.px(4.0), w - pad, y - p.px(3.0), BORDER);
        }
        y += p.px(4.0);
        side(&mut y, "Claude Desktop", "own sign-in", DESK, d);
    }

    if !m.others.is_empty() {
        if draw {
            p.fill(pad, y - p.px(4.0), w - pad, y - p.px(3.0), BORDER);
            p.text(pad, y + p.px(2.0), "OTHER ACCOUNTS", MUTED, SMALL);
        }
        y += p.px(24.0);
        for (email, u) in &m.others {
            if draw {
                let (figs, c) = match u {
                    Some(u) if u.week.u >= 100 => (format!("week full until {}", u.week.reset.map_or("?".into(), |t| clock(t, m.now))), RED),
                    Some(u) if u.five.u >= 100 => (format!("5h full until {}", u.five.reset.map_or("?".into(), |t| clock(t, m.now))), RED),
                    Some(u) => (format!("5h {}% · week {}%", u.five.u, u.week.u), if u.week.u >= 90 || u.five.u >= m.threshold { AMBER } else { GREEN }),
                    None => ("no figures yet".into(), MUTED),
                };
                let fw = p.width_of(&figs, SMALL);
                let name = email.split('@').next().unwrap_or(email);
                p.text(pad, y, &p.fitted(name, NORMAL, w - 2 * pad - fw - p.px(10.0)), TEXT, NORMAL);
                p.text_right(w - pad, y + p.px(1.5), &figs, c, SMALL);
            }
            y += p.px(19.0);
        }
        y += p.px(6.0);
    }

    if draw {
        let hint = if pinned { "Drag to move · double-click: dashboard" } else { "Click: dashboard · right-click the icon: menu" };
        p.text(pad, y, hint, MUTED, SMALL);
    }
    y += p.px(26.0);
    if draw {
        // Border last, over everything.
        let br = CreateSolidBrush(BORDER);
        FrameRect(dc, &RECT { left: 0, top: 0, right: w, bottom: y }, br);
        DeleteObject(br);
        for f in p.fonts {
            DeleteObject(f);
        }
    }
    y
}

#[cfg(test)]
mod tests {
    #[test]
    fn reads_both_sides_from_the_state() {
        let st = r#"{"now":1000,"settings":{"threshold":94},"activeId":"b","demo":false,
            "accounts":[{"id":"a","email":"a@x.com","usage":{"five":{"u":10,"resetsAt":2000},"week":{"u":100,"resetsAt":9000}}},
                        {"id":"b","email":"b@x.com","usage":null}],
            "desktop":{"found":true,"accounts":[{"label":"Account 1","email":null,"signedIn":true,"usage":null}]}}"#;
        let m = super::model(st);
        assert_eq!(m.cli.email.as_deref(), Some("b@x.com"));
        assert!(m.cli.usage.is_none());
        assert_eq!(m.desk.unwrap().email.as_deref(), Some("Account 1"));
        assert_eq!(m.others.len(), 1);
        assert_eq!(m.others[0].1.as_ref().unwrap().week.u, 100);
    }

    #[test]
    fn time_left() {
        assert_eq!(super::left(4 * 3_600_000 + 39 * 60_000), "4h39m");
        assert_eq!(super::left(51 * 3_600_000), "2d 3h");
        assert_eq!(super::left(30_000), "1m");
    }
}
