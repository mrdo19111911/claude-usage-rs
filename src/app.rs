// The running dashboard: its state, files and connections.

use crate::*;

pub(crate) struct App {
    pub(crate) st: Mutex<State>,
    pub(crate) demo: bool,
    pub(crate) data_file: PathBuf,
    pub(crate) creds_file: PathBuf,
    pub(crate) claude_json: PathBuf,
    pub(crate) keychain: bool,
    pub(crate) http: ureq::Agent,
    pub(crate) polling: AtomicBool,
    pub(crate) last_poll: AtomicI64,
    pub(crate) seq: AtomicU64,
    pub(crate) port: u16,
    pub(crate) desktop_dir: Option<PathBuf>,
    pub(crate) desktop_store: desktop::Store,
    pub(crate) switching: AtomicBool,
    pub(crate) login: Mutex<Option<login::Pending>>,
    // The email Claude Desktop is to be signed in to next, after a sign-out for it.
    pub(crate) desktop_expect: Mutex<Option<String>>,
    // Checking for and installing a newer release (update/).
    pub(crate) update: Mutex<update::Status>,
    // Gemini's thought signatures by tool_use id, for the next request (gemini/sigs.rs).
    pub(crate) gemini_sigs: gemini::Sigs,
    // Copy to add: the clipboard watch while the add dialog is open (gemini/watch.rs).
    pub(crate) gemini_watch: gemini::Watch,
    // The long record of usage answers, next to state.json (usage_log.rs).
    pub(crate) usage_log: usage_log::Log,
}

impl App {
    pub(crate) fn save(&self, st: &State) {
        if self.demo {
            return;
        }
        if let Some(dir) = self.data_file.parent() {
            let _ = fs::create_dir_all(dir);
        }
        let _ = write_private(&self.data_file, &serde_json::to_string_pretty(st).unwrap());
    }

    pub(crate) fn log(&self, st: &mut State, msg: String) {
        println!("{msg}");
        st.events.insert(0, Event { t: now(), msg });
        st.events.truncate(50);
    }

    pub(crate) fn new_id(&self) -> String {
        format!("{:x}{:x}", now(), self.seq.fetch_add(1, Ordering::Relaxed))
    }

    // ----- Claude Desktop -----
    pub(crate) fn data_dir(&self) -> &Path {
        self.data_file.parent().unwrap_or(Path::new("."))
    }
}
