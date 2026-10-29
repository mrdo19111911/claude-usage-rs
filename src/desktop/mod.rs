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

mod store;
mod switch;

pub use store::Store;
pub use switch::switch;

const COOKIE_FILES: [&str; 2] = ["Cookies", "Cookies-journal"];

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

/// Closing and starting Claude Desktop; a fake one stands in for it in tests.
pub trait App {
    fn stop(&self) -> Result<(), String>;
    fn start(&self) -> Result<(), String>;
}

// ---------- the real app (Windows) ----------
#[cfg(windows)]
pub use win::RealApp;

#[cfg(windows)]
mod win;
