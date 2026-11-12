// Loading state.json at start without losing it (docs/specs/state-load.md): a file that
// does not read is moved aside, never written over; one that cannot be read stops the start.
// Parts: Loaded, load, kept_note, App::load_saved.

use crate::*;
use std::path::{Path, PathBuf};

pub(crate) enum Loaded {
    Fresh,
    Read(State),
    Kept { aside: PathBuf, why: String },
}

pub(crate) fn load(path: &Path, stamp: i64) -> Result<Loaded, String> {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Loaded::Fresh),
        Err(e) => return Err(format!("{} could not be read: {e}", path.display())),
    };
    match serde_json::from_str(&text) {
        Ok(s) => Ok(Loaded::Read(s)),
        Err(why) => {
            let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let aside = path.with_file_name(format!("{name}.unreadable-{stamp}"));
            fs::rename(path, &aside).map_err(|e| format!("{} does not read ({why}) and could not be kept aside: {e}", path.display()))?;
            Ok(Loaded::Kept { aside, why: why.to_string() })
        }
    }
}

// The events-list note when state.json was kept aside, with the copy's path for support.
pub(crate) fn kept_note(aside: &Path) -> String {
    format!("Your saved accounts could not be loaded. A copy was kept; please contact support (file: {}).", aside.display())
}

impl App {
    // At start: the saved state, or a note that it was kept aside; a file that cannot be
    // read ends the run before anything saves over it.
    pub(crate) fn load_saved(&self, st: &mut State) {
        match load(&self.data_file, now()) {
            Ok(Loaded::Fresh) => {}
            Ok(Loaded::Read(s)) => *st = s,
            Ok(Loaded::Kept { aside, why }) => {
                // The parse error is for support only; the events list says it plainly.
                eprintln!("state.json does not read ({why}); kept as {}", aside.display());
                self.log(st, kept_note(&aside));
            }
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
    }
}

#[cfg(test)]
#[path = "state_load_tests.rs"]
mod tests;
