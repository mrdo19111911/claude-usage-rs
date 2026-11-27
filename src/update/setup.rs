// Around running the setup, the pure parts (docs/specs/update.md, "Install"): whether
// this copy's setup asks for admin, what the setup's end means, emptying the update folder.

use super::*;

// What a setup that did not install says; the exit code or start error goes to stderr.
pub(crate) const NOT_INSTALLED: &str = "The update was not installed. You can try again.";

// Whether `exe` lives under one of the machine-wide program folders: an all-users
// install, whose setup asks for admin (UAC). Whole path parts, compared without case.
pub(crate) fn needs_admin(exe: &Path, program_dirs: &[PathBuf]) -> bool {
    let parts = |p: &Path| p.components().map(|c| c.as_os_str().to_string_lossy().to_lowercase()).collect::<Vec<_>>();
    let exe = parts(exe);
    program_dirs.iter().map(|d| parts(d)).any(|d| !d.is_empty() && exe.len() > d.len() && exe[..d.len()] == d[..])
}

// The machine-wide program folders of this Windows (none set elsewhere).
pub(crate) fn program_dirs() -> Vec<PathBuf> {
    ["ProgramFiles", "ProgramFiles(x86)", "ProgramW6432"]
        .iter()
        .filter_map(|v| std::env::var_os(v))
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .collect()
}

// Whether a found version is installed with no click: on a build that installs, and
// only a copy whose setup needs no admin (a UAC prompt must follow a click).
pub(crate) fn installs_by_itself(installs: bool, exe: Option<&Path>, program_dirs: &[PathBuf]) -> bool {
    installs && !exe.is_some_and(|e| needs_admin(e, program_dirs))
}

// The same, for this running copy.
pub(crate) fn installs_itself() -> bool {
    installs_by_itself(INSTALLS, std::env::current_exe().ok().as_deref(), &program_dirs())
}

// The setup's end: its exit code (None: ended without one), or why it did not start.
// Only 0 is an install; the setup then closes this app itself.
pub(crate) fn setup_result(exit: Result<Option<i32>, String>) -> Result<(), String> {
    match exit {
        Ok(Some(0)) => Ok(()),
        Ok(code) => {
            eprintln!("The setup ended with exit code {code:?}.");
            Err(NOT_INSTALLED.into())
        }
        Err(e) => {
            eprintln!("{e}");
            Err(NOT_INSTALLED.into())
        }
    }
}

// Deletes the files directly in `dir` (older setups, .part, setup.log); folders below
// it and anything that will not go are left.
pub(crate) fn clear_dir(dir: &Path) {
    for e in fs::read_dir(dir).into_iter().flatten().flatten() {
        if e.file_type().is_ok_and(|t| t.is_file()) {
            let _ = fs::remove_file(e.path());
        }
    }
}
