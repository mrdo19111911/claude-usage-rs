// Closing and starting the real Claude Desktop (Windows).

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
        let parent: HashMap<u32, u32> = processes().iter().map(|p| (p.pid, p.parent)).collect();
        super::ancestry::descends_from(&parent, super::ancestry::started_at, std::process::id(), &self.claude_pids())
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
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let ok = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", script])
            .creation_flags(CREATE_NO_WINDOW)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ok { Ok(()) } else { Err("could not start Claude Desktop; start it from the Start menu".into()) }
    }
}
