// Claude Desktop's sign-in, and switching it between saved accounts (Windows).
//
// The app keeps its sign-in in two places in its data folder: the `oauth:*` keys and
// `lastKnownAccountUuid` of config.json (the tokens encrypted by the app for this
// Windows user), and its cookie store Network\Cookies. A switch closes the app, saves
// the sign-in it holds under its account, puts the other account's back and starts
// the app again. Nothing is decrypted: the saved values are copied as they are.
// Same method as plexxed/claude-desktop-account-switcher (scripts/Private/Switch.ps1).

use serde_json::{Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

const COOKIE_FILES: [&str; 2] = ["Cookies", "Cookies-journal"];
const BACKUPS_KEPT: usize = 5;

/// The app's data folder: the first known place that holds config.json.
/// Since app 3.6556 the MSIX build keeps it in %LOCALAPPDATA%\Claude-Data\claude_<id>;
/// older builds in the package's LocalCache\Roaming\Claude or in %APPDATA%\Claude.
pub fn find_data_dir() -> Option<PathBuf> {
    if let Some(d) = std::env::var_os("CUD_DESKTOP_DIR") {
        let d = PathBuf::from(d);
        return d.join("config.json").is_file().then_some(d);
    }
    let mut candidates = vec![];
    if let Some(local) = std::env::var_os("LOCALAPPDATA").map(PathBuf::from) {
        candidates.extend(subdirs(&local.join("Claude-Data"), "claude_"));
        candidates.extend(subdirs(&local.join("Packages"), "Claude_").into_iter().map(|p| p.join("LocalCache").join("Roaming").join("Claude")));
    }
    if let Some(roaming) = std::env::var_os("APPDATA") {
        candidates.push(PathBuf::from(roaming).join("Claude"));
    }
    candidates.into_iter().find(|d| d.join("config.json").is_file())
}

fn subdirs(dir: &Path, prefix: &str) -> Vec<PathBuf> {
    let Ok(rd) = fs::read_dir(dir) else { return vec![] };
    let mut v: Vec<PathBuf> = rd
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()) && e.file_name().to_string_lossy().starts_with(prefix))
        .map(|e| e.path())
        .collect();
    v.sort();
    v
}

fn is_sign_in_key(k: &str) -> bool {
    k.starts_with("oauth:") || k == "lastKnownAccountUuid"
}

/// An account id as the app writes it; anything else is refused before it names a folder.
pub fn valid_account_id(s: &str) -> bool {
    s.len() == 36 && s.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-')
}

fn read_config(dir: &Path) -> Result<Map<String, Value>, String> {
    let text = fs::read_to_string(dir.join("config.json")).map_err(|e| format!("config.json: {e}"))?;
    match serde_json::from_str(&text) {
        Ok(Value::Object(m)) => Ok(m),
        _ => Err("config.json is not a JSON object".into()),
    }
}

fn write_config(dir: &Path, config: &Map<String, Value>) -> Result<(), String> {
    crate::write_private(&dir.join("config.json"), &serde_json::to_string_pretty(config).unwrap()).map_err(|e| format!("config.json: {e}"))
}

/// The account the app is signed in to, or None when it is signed out.
pub fn signed_in(dir: &Path) -> Option<String> {
    let config = read_config(dir).ok()?;
    signed_in_of(&config)
}

fn signed_in_of(config: &Map<String, Value>) -> Option<String> {
    if !config.keys().any(|k| k.starts_with("oauth:")) {
        return None;
    }
    config.get("lastKnownAccountUuid").and_then(Value::as_str).filter(|s| valid_account_id(s)).map(str::to_string)
}

fn copy_cookies(from: &Path, to: &Path) -> Result<(), String> {
    fs::create_dir_all(to).map_err(|e| e.to_string())?;
    for f in COOKIE_FILES {
        let (src, dst) = (from.join(f), to.join(f));
        if src.is_file() {
            fs::copy(&src, &dst).map_err(|e| format!("{}: {e}", src.display()))?;
        } else if dst.exists() {
            fs::remove_file(&dst).map_err(|e| format!("{}: {e}", dst.display()))?;
        }
    }
    Ok(())
}

/// Saved sign-ins live in `<store>/profiles/<account>/` (auth.json and the cookie
/// store), backups of the app's own in `<store>/backups/<time>/`.
pub struct Store {
    pub root: PathBuf,
}

impl Store {
    pub fn profile(&self, account: &str) -> PathBuf {
        self.root.join("profiles").join(account)
    }

    pub fn has_sign_in(&self, account: &str) -> bool {
        self.profile(account).join("auth.json").is_file()
    }

    pub fn forget(&self, account: &str) -> Result<(), String> {
        let p = self.profile(account);
        if p.exists() {
            fs::remove_dir_all(&p).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    fn save(&self, dir: &Path, config: &Map<String, Value>, account: &str) -> Result<(), String> {
        let auth: Map<String, Value> = config.iter().filter(|(k, _)| is_sign_in_key(k)).map(|(k, v)| (k.clone(), v.clone())).collect();
        let p = self.profile(account);
        fs::create_dir_all(&p).map_err(|e| e.to_string())?;
        crate::write_private(&p.join("auth.json"), &serde_json::to_string_pretty(&auth).unwrap()).map_err(|e| e.to_string())?;
        copy_cookies(&dir.join("Network"), &p)
    }

    fn backup(&self, dir: &Path, stamp: i64) -> Result<PathBuf, String> {
        let root = self.root.join("backups");
        let b = root.join(stamp.to_string());
        fs::create_dir_all(&b).map_err(|e| e.to_string())?;
        fs::copy(dir.join("config.json"), b.join("config.json")).map_err(|e| format!("backup: {e}"))?;
        copy_cookies(&dir.join("Network"), &b)?;
        let mut all = subdirs(&root, "");
        all.sort_by_key(|p| p.file_name().and_then(|n| n.to_str()).and_then(|n| n.parse::<i64>().ok()).unwrap_or(0));
        for old in all.iter().rev().skip(BACKUPS_KEPT) {
            let _ = fs::remove_dir_all(old);
        }
        Ok(b)
    }
}

/// Removes the app's sign-in from `config` and puts `auth`'s in (none: signed out).
fn put_sign_in(config: &mut Map<String, Value>, auth: Option<&Map<String, Value>>) {
    config.retain(|k, _| !is_sign_in_key(k));
    if let Some(a) = auth {
        for (k, v) in a.iter().filter(|(k, _)| is_sign_in_key(k)) {
            config.insert(k.clone(), v.clone());
        }
    }
}

/// Closing and starting Claude Desktop; a fake one stands in for it in tests.
pub trait App {
    fn stop(&self) -> Result<(), String>;
    fn start(&self) -> Result<(), String>;
}

/// The switch itself. `target`: the account to sign in to, or None to leave the app
/// signed out (its sign-in screen then adds another account). Returns the account
/// whose sign-in was saved on the way out. A failure after the backup puts the app's
/// sign-in back as it was; the app is started again whatever happened.
pub fn switch(dir: &Path, store: &Store, target: Option<&str>, app: &dyn App, stamp: i64) -> Result<Option<String>, String> {
    let target_auth = match target {
        Some(t) => {
            if !valid_account_id(t) {
                return Err("not an account id".into());
            }
            let text = fs::read_to_string(store.profile(t).join("auth.json")).map_err(|_| "no saved sign-in for this account".to_string())?;
            match serde_json::from_str(&text) {
                Ok(Value::Object(m)) => Some(m),
                _ => return Err("saved sign-in is unreadable".into()),
            }
        }
        None => None,
    };
    let stopped = app.stop();
    let res = stopped.and_then(|_| {
        let backup = store.backup(dir, stamp)?;
        let swap = || -> Result<Option<String>, String> {
            // Read only now: the app may write its sign-in while it quits.
            let mut config = read_config(dir)?;
            let current = signed_in_of(&config);
            if let Some(c) = &current {
                store.save(dir, &config, c)?;
            }
            put_sign_in(&mut config, target_auth.as_ref());
            write_config(dir, &config)?;
            match target {
                Some(t) => copy_cookies(&store.profile(t), &dir.join("Network"))?,
                None => copy_cookies(&store.root.join("no-cookies"), &dir.join("Network"))?,
            }
            Ok(current)
        };
        swap().map_err(|e| {
            let restored = fs::copy(backup.join("config.json"), dir.join("config.json")).map_err(|x| x.to_string()).and_then(|_| copy_cookies(&backup, &dir.join("Network")));
            match restored {
                Ok(()) => format!("{e} (sign-in put back as it was)"),
                Err(x) => format!("{e}; putting the sign-in back failed too: {x}, backup in {}", backup.display()),
            }
        })
    });
    let started = app.start();
    let saved = res?;
    started?;
    Ok(saved)
}

// ---------- the real app (Windows) ----------
#[cfg(windows)]
pub use win::RealApp;

#[cfg(windows)]
mod win {
    use std::collections::HashMap;
    use std::os::windows::ffi::OsStringExt;
    use std::path::PathBuf;
    use std::process::Command;
    use std::thread::sleep;
    use std::time::{Duration, Instant};
    use windows_sys::Win32::Foundation::{CloseHandle, HWND, INVALID_HANDLE_VALUE, LPARAM, WPARAM};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS};
    use windows_sys::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, TerminateProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE};
    use windows_sys::Win32::UI::WindowsAndMessaging::{EnumWindows, GetWindowThreadProcessId, SendMessageTimeoutW, SMTO_ABORTIFHUNG, WM_ENDSESSION, WM_QUERYENDSESSION};

    const ENDSESSION_CLOSEAPP: LPARAM = 1;
    const QUIT_GRACE: Duration = Duration::from_secs(30);
    const KILL_WAIT: Duration = Duration::from_secs(20);

    pub struct RealApp {
        pub data_dir: PathBuf,
    }

    struct Proc {
        pid: u32,
        parent: u32,
        name: String,
    }

    fn processes() -> Vec<Proc> {
        let mut out = vec![];
        unsafe {
            let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snap == INVALID_HANDLE_VALUE {
                return out;
            }
            let mut e: PROCESSENTRY32W = std::mem::zeroed();
            e.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
            let mut ok = Process32FirstW(snap, &mut e) != 0;
            while ok {
                let len = e.szExeFile.iter().position(|&c| c == 0).unwrap_or(e.szExeFile.len());
                out.push(Proc { pid: e.th32ProcessID, parent: e.th32ParentProcessID, name: String::from_utf16_lossy(&e.szExeFile[..len]) });
                ok = Process32NextW(snap, &mut e) != 0;
            }
            CloseHandle(snap);
        }
        out
    }

    fn image_path(pid: u32) -> Option<PathBuf> {
        unsafe {
            let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if h.is_null() {
                return None;
            }
            let mut buf = [0u16; 1024];
            let mut len = buf.len() as u32;
            let ok = QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, buf.as_mut_ptr(), &mut len) != 0;
            CloseHandle(h);
            ok.then(|| PathBuf::from(std::ffi::OsString::from_wide(&buf[..len as usize])))
        }
    }

    impl RealApp {
        /// The app's processes: the app itself (MSIX or per-user install) and the
        /// Claude Code it runs from its data folder.
        fn claude_pids(&self) -> Vec<u32> {
            let installed = std::env::var_os("LOCALAPPDATA").map(|l| PathBuf::from(l).join("AnthropicClaude"));
            processes()
                .into_iter()
                .filter(|p| p.name.eq_ignore_ascii_case("claude.exe"))
                .filter(|p| {
                    image_path(p.pid).is_some_and(|path| {
                        let s = path.to_string_lossy().to_lowercase();
                        s.contains("\\windowsapps\\claude_") || path.starts_with(&self.data_dir) || installed.as_ref().is_some_and(|i| path.starts_with(i))
                    })
                })
                .map(|p| p.pid)
                .collect()
        }

        /// Whether this dashboard runs inside the app's process tree: closing the app
        /// would then end the dashboard half-way through a switch.
        pub fn inside_claude(&self) -> bool {
            let all = processes();
            let parent: HashMap<u32, u32> = all.iter().map(|p| (p.pid, p.parent)).collect();
            let claude = self.claude_pids();
            let mut pid = std::process::id();
            for _ in 0..64 {
                match parent.get(&pid) {
                    Some(&p) if p != 0 && p != pid => {
                        if claude.contains(&p) {
                            return true;
                        }
                        pid = p;
                    }
                    _ => break,
                }
            }
            false
        }

        fn wait_gone(&self, d: Duration) -> bool {
            let end = Instant::now() + d;
            while !self.claude_pids().is_empty() {
                if Instant::now() > end {
                    return false;
                }
                sleep(Duration::from_millis(300));
            }
            true
        }
    }

    struct Found {
        pids: Vec<u32>,
        windows: Vec<HWND>,
    }

    unsafe extern "system" fn collect(w: HWND, l: LPARAM) -> i32 {
        let f = &mut *(l as *mut Found);
        let mut pid = 0u32;
        GetWindowThreadProcessId(w, &mut pid);
        if f.pids.contains(&pid) {
            f.windows.push(w);
        }
        1
    }

    /// Asks the windows of `pids` to end their session, as Windows does when an
    /// installer must close an app: the app quits as from its tray menu, saving its
    /// state. False when a window refused or there was none.
    fn ask_to_quit(pids: &[u32]) -> bool {
        let mut f = Found { pids: pids.to_vec(), windows: vec![] };
        unsafe {
            EnumWindows(Some(collect), &mut f as *mut Found as LPARAM);
            let mut agreed = true;
            for &w in &f.windows {
                let mut answer = 0usize;
                let answered = SendMessageTimeoutW(w, WM_QUERYENDSESSION, 0, ENDSESSION_CLOSEAPP, SMTO_ABORTIFHUNG, 5000, &mut answer) != 0;
                if answered && answer == 0 {
                    agreed = false;
                }
            }
            for &w in &f.windows {
                let mut answer = 0usize;
                SendMessageTimeoutW(w, WM_ENDSESSION, agreed as WPARAM, ENDSESSION_CLOSEAPP, SMTO_ABORTIFHUNG, 5000, &mut answer);
            }
            agreed && !f.windows.is_empty()
        }
    }

    impl super::App for RealApp {
        fn stop(&self) -> Result<(), String> {
            let pids = self.claude_pids();
            if pids.is_empty() {
                return Ok(());
            }
            if !(ask_to_quit(&pids) && self.wait_gone(QUIT_GRACE)) {
                for pid in self.claude_pids() {
                    unsafe {
                        let h = OpenProcess(PROCESS_TERMINATE, 0, pid);
                        if !h.is_null() {
                            TerminateProcess(h, 1);
                            CloseHandle(h);
                        }
                    }
                }
                if !self.wait_gone(KILL_WAIT) {
                    return Err("Claude Desktop did not close".into());
                }
            }
            // Let the last file handles go.
            sleep(Duration::from_millis(800));
            Ok(())
        }

        fn start(&self) -> Result<(), String> {
            let script = "$p = Get-AppxPackage -Name Claude | Select-Object -First 1; \
                if ($p) { $id = @((Get-AppxPackageManifest $p).Package.Applications.Application)[0].Id; \
                Start-Process ('shell:AppsFolder\\' + $p.PackageFamilyName + '!' + $id); exit 0 }; \
                $e = Join-Path $env:LOCALAPPDATA 'AnthropicClaude\\claude.exe'; \
                if (Test-Path -LiteralPath $e) { Start-Process $e; exit 0 }; exit 3";
            let ok = Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command", script])
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            if ok { Ok(()) } else { Err("could not start Claude Desktop; start it from the Start menu".into()) }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::cell::RefCell;

    const A: &str = "aaaaaaaa-0000-0000-0000-000000000001";
    const B: &str = "bbbbbbbb-0000-0000-0000-000000000002";

    struct Fake {
        calls: RefCell<Vec<&'static str>>,
        fail_stop: bool,
    }
    impl App for Fake {
        fn stop(&self) -> Result<(), String> {
            self.calls.borrow_mut().push("stop");
            if self.fail_stop { Err("stuck".into()) } else { Ok(()) }
        }
        fn start(&self) -> Result<(), String> {
            self.calls.borrow_mut().push("start");
            Ok(())
        }
    }
    fn fake() -> Fake {
        Fake { calls: RefCell::new(vec![]), fail_stop: false }
    }

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("cud-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(d.join("app").join("Network")).unwrap();
        d
    }

    fn sign_in(app: &Path, account: &str, token: &str, cookie: &str) {
        let cfg = json!({ "zFirst": 1, "oauth:tokenCache": format!("v10{token}"), "lastKnownAccountUuid": account, "dxt:allowlist": true, "oauth:tokenCacheV2": "v10x" });
        fs::write(app.join("config.json"), cfg.to_string()).unwrap();
        fs::write(app.join("Network").join("Cookies"), cookie).unwrap();
    }

    fn config(app: &Path) -> Map<String, Value> {
        read_config(app).unwrap()
    }

    #[test]
    fn switches_back_and_forth_keeping_other_settings_and_key_order() {
        let d = tmp("swap");
        let (app, store) = (d.join("app"), Store { root: d.join("store") });
        sign_in(&app, A, "tokA", "cookieA");
        // First switch to a new account: A is saved, the app is left signed out.
        assert_eq!(switch(&app, &store, None, &fake(), 1).unwrap().as_deref(), Some(A));
        assert!(store.has_sign_in(A));
        assert_eq!(signed_in(&app), None);
        assert!(!app.join("Network").join("Cookies").exists());
        assert_eq!(config(&app).get("dxt:allowlist"), Some(&json!(true)));
        // The person signs in to B in the app; switching to A saves B and restores A.
        sign_in(&app, B, "tokB", "cookieB");
        let f = fake();
        assert_eq!(switch(&app, &store, Some(A), &f, 2).unwrap().as_deref(), Some(B));
        assert_eq!(*f.calls.borrow(), vec!["stop", "start"]);
        assert_eq!(signed_in(&app).as_deref(), Some(A));
        assert_eq!(config(&app)["oauth:tokenCache"], json!("v10tokA"));
        assert_eq!(fs::read_to_string(app.join("Network").join("Cookies")).unwrap(), "cookieA");
        let keys: Vec<String> = config(&app).keys().cloned().collect();
        assert_eq!(keys.first().map(String::as_str), Some("zFirst"), "untouched keys keep their order");
        // And back to B.
        switch(&app, &store, Some(B), &fake(), 3).unwrap();
        assert_eq!(signed_in(&app).as_deref(), Some(B));
        assert_eq!(fs::read_to_string(app.join("Network").join("Cookies")).unwrap(), "cookieB");
    }

    #[test]
    fn saves_the_sign_in_as_it_is_after_the_app_closed() {
        // The app refreshes its token while quitting: the saved copy must be that one.
        struct Rotating(PathBuf);
        impl App for Rotating {
            fn stop(&self) -> Result<(), String> {
                sign_in(&self.0, A, "tokA-new", "cookieA");
                Ok(())
            }
            fn start(&self) -> Result<(), String> {
                Ok(())
            }
        }
        let d = tmp("rotate");
        let (app, store) = (d.join("app"), Store { root: d.join("store") });
        sign_in(&app, A, "tokA-old", "cookieA");
        switch(&app, &store, None, &Rotating(app.clone()), 1).unwrap();
        let saved = fs::read_to_string(store.profile(A).join("auth.json")).unwrap();
        assert!(saved.contains("tokA-new") && !saved.contains("tokA-old"));
    }

    #[test]
    fn refuses_unknown_or_bad_targets_without_closing_the_app() {
        let d = tmp("refuse");
        let (app, store) = (d.join("app"), Store { root: d.join("store") });
        sign_in(&app, A, "tokA", "cookieA");
        let f = fake();
        assert!(switch(&app, &store, Some(B), &f, 1).is_err());
        assert!(switch(&app, &store, Some("..\\..\\x"), &f, 1).is_err());
        assert!(f.calls.borrow().is_empty());
    }

    #[test]
    fn a_failed_close_leaves_the_sign_in_alone_and_restarts() {
        let d = tmp("stuck");
        let (app, store) = (d.join("app"), Store { root: d.join("store") });
        sign_in(&app, A, "tokA", "cookieA");
        switch(&app, &store, None, &fake(), 1).unwrap();
        sign_in(&app, B, "tokB", "cookieB");
        let f = Fake { calls: RefCell::new(vec![]), fail_stop: true };
        assert!(switch(&app, &store, Some(A), &f, 2).is_err());
        assert_eq!(*f.calls.borrow(), vec!["stop", "start"]);
        assert_eq!(signed_in(&app).as_deref(), Some(B));
    }

    #[test]
    fn keeps_the_last_backups_only() {
        let d = tmp("backups");
        let (app, store) = (d.join("app"), Store { root: d.join("store") });
        sign_in(&app, A, "tokA", "cookieA");
        for i in 0..8 {
            store.backup(&app, i).unwrap();
        }
        let left: Vec<String> = subdirs(&store.root.join("backups"), "").iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect();
        assert_eq!(left, vec!["3", "4", "5", "6", "7"]);
    }

    #[test]
    fn finds_the_relocated_data_folder() {
        let d = tmp("find");
        let local = d.join("local");
        let moved = local.join("Claude-Data").join("claude_pzs8sxrjxfjjc");
        fs::create_dir_all(&moved).unwrap();
        fs::write(moved.join("config.json"), "{}").unwrap();
        fs::create_dir_all(local.join("Packages").join("Claude_pzs8sxrjxfjjc").join("LocalCache").join("Roaming").join("Claude")).unwrap();
        assert_eq!(subdirs(&local.join("Claude-Data"), "claude_"), vec![moved]);
    }
}
