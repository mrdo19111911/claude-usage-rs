// "Use this" on the card (POST /api/gemini/open): a new console window running cgemini,
// started in the user's home folder, so nothing has to be typed.

use crate::*;
use super::script;

// cmd.exe's command line: run the script, keep the window when Claude Code ends.
pub(super) fn args(script: &Path) -> String {
    format!("/k \"{}\"", script.display())
}

pub(super) fn open(app: &Arc<App>, _body: &Value) -> Result<(), String> {
    {
        // In turn with a save (setup.rs): never the token of a key just replaced.
        let _one = super::setup::lock();
        let cfg = app.st.lock().unwrap().gemini.clone().ok_or("Add a Gemini account first")?;
        // Written at start and on save; once more in case it was deleted since.
        script::write(app, &cfg).map_err(|e| format!("cgemini.cmd could not be written: {e}"))?;
    }
    let home = std::env::var_os("USERPROFILE").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    spawn(&home, &args(&script::script_path(app)))
}

// Out of this app's job when allowed (a job may end its processes with it), as the
// update setup is; a console of its own.
#[cfg(all(windows, not(test)))]
fn spawn(dir: &Path, line: &str) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
    const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x0100_0000;
    let run = |flags: u32| Command::new("cmd.exe").raw_arg(line).current_dir(dir).creation_flags(CREATE_NEW_CONSOLE | flags).spawn();
    run(CREATE_BREAKAWAY_FROM_JOB).or_else(|_| run(0)).map(|_| ()).map_err(|e| format!("Could not open the window: {e}"))
}

#[cfg(all(not(windows), not(test)))]
fn spawn(_: &Path, _: &str) -> Result<(), String> {
    Err("Only on Windows".into())
}

// Tests: what would have opened, on this thread.
