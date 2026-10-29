// The cgemini folder on the user's PATH: HKCU\Environment `Path`, then a broadcast so
// terminals opened afterwards see it. As rustup and Scoop do it (docs/SO_SANH_GEMINI_PROXY.md,
// "Proxy và PATH"), but added at the end, so it never hides another program.

use std::path::Path;

// Same folder whatever the case or a trailing backslash (Windows paths ignore both).
fn same_dir(a: &str, b: &str) -> bool {
    let t = |s: &str| s.trim().trim_end_matches(['\\', '/']).to_lowercase();
    !a.trim().is_empty() && t(a) == t(b)
}

// `old` with `dir` at the end, unless it is there already.
pub(crate) fn path_with(old: &str, dir: &str) -> String {
    if old.split(';').any(|p| same_dir(p, dir)) {
        return old.to_string();
    }
    let rest = old.trim_end_matches(';');
    if rest.is_empty() {
        dir.to_string()
    } else {
        format!("{rest};{dir}")
    }
}

// `old` without `dir` (every copy); the other entries exactly as they were.
pub(crate) fn path_without(old: &str, dir: &str) -> String {
    if !old.split(';').any(|p| same_dir(p, dir)) {
        return old.to_string();
    }
    old.split(';').filter(|p| !same_dir(p, dir)).collect::<Vec<_>>().join(";")
}

pub(crate) fn add(dir: &Path) -> Result<(), String> {
    change(|old| path_with(old, &dir.display().to_string()))
}

pub(crate) fn remove(dir: &Path) -> Result<(), String> {
    change(|old| path_without(old, &dir.display().to_string()))
}

// Elsewhere than Windows: no user PATH of this kind to change.
#[cfg(all(not(windows), not(test)))]
fn change(edit: impl Fn(&str) -> String) -> Result<(), String> {
    let _ = edit;
    Ok(())
}

// Tests never touch the real PATH: nothing happens, or, with FAIL set on this thread, a
// PATH that cannot be changed.

#[cfg(all(windows, not(test)))]
fn change(edit: impl Fn(&str) -> String) -> Result<(), String> {
    super::path_win::change(edit)
}
