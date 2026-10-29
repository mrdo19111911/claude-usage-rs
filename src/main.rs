// Claude usage dashboard: theo dõi % usage 5 giờ / tuần của nhiều tài khoản Claude
// và tự chuyển login của Claude Code sang tài khoản còn dư khi chạm ngưỡng.
// Chạy file là xong: tự mở trình duyệt, tự nhận tài khoản mỗi lần bạn /login.
// Claude Desktop (Windows) giữ đăng nhập riêng: desktop.rs đổi nó khi bạn bấm chuyển.

// No console window of its own on Windows (not even for the SessionStart hook); from a
// terminal it writes to that terminal (main). It lives in the tray instead (tray.rs).
#![cfg_attr(windows, windows_subsystem = "windows")]

mod desktop;
mod gemini;
mod lock;
mod login;
#[cfg(windows)]
mod tray;
mod web;

// The core, one concern per file (docs/specs/modules.md). Each is glob-imported here and
// each starts with `use crate::*;`, so code moved out of this file kept its names.
mod anthropic;
mod app;
mod creds;
mod model;
mod poll;
mod port;
mod server;
mod signin;
mod state_json;
mod switch;
mod sync;
mod update;
mod util;
use anthropic::*;
use app::*;
use creds::*;
use model::*;
use poll::*;
use server::*;
use signin::*;
use state_json::*;
use sync::*;
use util::*;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn main() {
    #[cfg(windows)]
    unsafe {
        windows_sys::Win32::System::Console::AttachConsole(windows_sys::Win32::System::Console::ATTACH_PARENT_PROCESS);
    }
    let args: Vec<String> = std::env::args().collect();
    let demo = args.iter().any(|a| a == "--demo");
    // Windows: the tray icon is the way in, so no browser tab unless asked for (--open).
    let no_open = if cfg!(windows) { !args.iter().any(|a| a == "--open") } else { args.iter().any(|a| a == "--no-open") };
    // Windows: any free port (0; port.rs takes the last run's again), since the tray icon
    // knows the address; elsewhere it is typed or bookmarked, so a fixed one. PORT sets it.
    let port: u16 = std::env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(if cfg!(windows) { 0 } else { 4777 });

    let config_dir = std::env::var_os("CLAUDE_CONFIG_DIR").map(PathBuf::from);
    let claude_dir = config_dir.clone().unwrap_or_else(|| home().join(".claude"));
    let data_dir = std::env::var_os("CUD_DATA_DIR").map(PathBuf::from).unwrap_or_else(|| home().join(".claude-usage-dashboard"));

    // As a SessionStart hook: silent, and never fails the session it runs in.
    if args.iter().any(|a| a == "--note-session") {
        if std::env::var("CLAUDE_CODE_ENTRYPOINT").as_deref() == Ok("claude-desktop") {
            if let (Ok(u), Ok(e)) = (std::env::var("CLAUDE_CODE_ACCOUNT_UUID"), std::env::var("CLAUDE_CODE_USER_EMAIL")) {
                note_session(&data_dir, &u, &e);
            }
        }
        return;
    }

    // One dashboard at a time; `--quit` (the installer) ends the running one instead.
    #[cfg(windows)]
    if !tray::claim_instance(demo, args.iter().any(|a| a == "--quit")) {
        return;
    }

    let server = match port::listen(&data_dir, port, !demo) {
        Ok(s) => s,
        Err(e) => {
            // Most likely it is already running: just open it.
            eprintln!("Port {port} busy ({e}); opening the existing dashboard.");
            if !no_open {
                open_browser(&format!("http://127.0.0.1:{port}"));
            }
            return;
        }
    };
    let port = server.server_addr().to_ip().map_or(port, |a| a.port());

    let app = Arc::new(App {
        st: Mutex::new(State::default()),
        demo,
        data_file: data_dir.join("state.json"),
        creds_file: claude_dir.join(".credentials.json"),
        claude_json: match &config_dir {
            Some(d) => d.join(".claude.json"),
            None => home().join(".claude.json"),
        },
        keychain: cfg!(target_os = "macos") && std::env::var_os("CUD_NO_KEYCHAIN").is_none(),
        http: ureq::AgentBuilder::new().timeout(Duration::from_secs(20)).build(),
        polling: AtomicBool::new(false),
        last_poll: AtomicI64::new(0),
        seq: AtomicU64::new(0),
        port,
        desktop_dir: if demo { None } else { desktop::find_data_dir() },
        desktop_store: desktop::Store { root: data_dir.join("desktop") },
        switching: AtomicBool::new(false),
        login: Mutex::new(None),
        desktop_expect: Mutex::new(None),
        update: Mutex::new(update::Status::default()),
        gemini_sigs: Default::default(),
        gemini_watch: Default::default(),
    });

    {
        let mut st = app.st.lock().unwrap();
        if demo {
            *st = demo_state(&app);
        } else if let Some(s) = fs::read_to_string(&app.data_file).ok().and_then(|s| serde_json::from_str(&s).ok()) {
            *st = s;
        }
    }
    gemini::start(&app); // cgemini.cmd again, for this run's port

    let url = format!("http://127.0.0.1:{port}");
    println!("Claude usage dashboard{}: {url}", if demo { " (demo)" } else { "" });
    if !demo {
        println!("Data: {}", app.data_file.display());
        println!("Claude Code login: {}", if app.keychain { "macOS Keychain".into() } else { app.creds_file.display().to_string() });
        println!("Claude Desktop data: {}", app.desktop_dir.as_ref().map(|d| d.display().to_string()).unwrap_or_else(|| "not found".into()));
    }
    if !no_open {
        open_browser(&url);
    }
    #[cfg(windows)]
    {
        println!("Running in the tray: hover its icon for the figures, right-click it to quit.");
        // Let go of the terminal: closing it or Ctrl+C there would kill us, icon left behind.
        // Its stdio handles die with it: cleared, or every spawn (browser, PowerShell) fails.
        unsafe {
            use windows_sys::Win32::System::Console::*;
            FreeConsole();
            for h in [STD_INPUT_HANDLE, STD_OUTPUT_HANDLE, STD_ERROR_HANDLE] {
                SetStdHandle(h, std::ptr::null_mut());
            }
        }
        let (state, act) = app.tray_hooks();
        tray::start(state, act, url.clone(), data_dir.clone());
    }
    #[cfg(not(windows))]
    println!("Keep this window open. Ctrl+C to stop.");

    update::start(app.clone());
    let poller = app.clone();
    thread::spawn(move || loop {
        poller.poll_all(false);
        let start = now();
        let mut last_sync = start;
        loop {
            thread::sleep(Duration::from_secs(1));
            // Each account has its own next check (next_check_at); look for due ones.
            if now() - start >= 5_000 {
                break;
            }
            // Notice a fresh /login (or Desktop sign-in) quickly instead of waiting a whole poll cycle.
            if !poller.demo && now() - last_sync >= 10_000 {
                last_sync = now();
                let before = poller.st.lock().unwrap().accounts.len();
                poller.sync_active();
                let mut st = poller.st.lock().unwrap();
                poller.sync_desktop(&mut st);
                if st.accounts.len() != before {
                    break;
                }
            }
        }
    });

    for rq in server.incoming_requests() {
        let app = app.clone();
        thread::spawn(move || handle(&app, rq));
    }
}
