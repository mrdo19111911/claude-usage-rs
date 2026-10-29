// Saved sign-ins of Claude Desktop, and backups of its own.

use super::{copy_cookies, is_sign_in_key, subdirs};
use serde_json::{Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

const BACKUPS_KEPT: usize = 5;

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

    pub(super) fn save(&self, dir: &Path, config: &Map<String, Value>, account: &str) -> Result<(), String> {
        let auth: Map<String, Value> = config.iter().filter(|(k, _)| is_sign_in_key(k)).map(|(k, v)| (k.clone(), v.clone())).collect();
        let p = self.profile(account);
        fs::create_dir_all(&p).map_err(|e| e.to_string())?;
        crate::write_private(&p.join("auth.json"), &serde_json::to_string_pretty(&auth).unwrap()).map_err(|e| e.to_string())?;
        copy_cookies(&dir.join("Network"), &p)
    }

    pub(super) fn backup(&self, dir: &Path, stamp: i64) -> Result<PathBuf, String> {
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
