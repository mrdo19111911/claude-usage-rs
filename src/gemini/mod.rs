// "Claude shell, Gemini inside": the `cgemini` command (docs/specs/gemini.md).
// Claude Code, pointed at this app by ANTHROPIC_BASE_URL, sends Anthropic Messages
// requests to /v1/*; they go to Google's Gemini API with the user's AI Studio key.
// Parts: request (Anthropic → Gemini body), schema (tool schemas), sigs (thought
// signatures by tool_use id), stream (Gemini chunks → Anthropic SSE), errors (Google's
// errors → Anthropic's), proxy (/v1/* and the call to Google), api (/api/gemini/* for
// the page), script (cgemini.cmd and the user's PATH).

use crate::*;

mod api;
mod errors;
mod proxy;
mod request;
mod schema;
mod script;
mod sigs;
mod stream;

pub(crate) use api::{api, public};
pub(crate) use proxy::serve;
pub(crate) use sigs::Sigs;

// Main model and the fast one (Claude Code's "haiku" calls), as the research chose.
pub(crate) const MODEL: &str = "gemini-3.8-flash";
pub(crate) const FAST_MODEL: &str = "gemini-3.5-flash-lite";

// Saved in state.json (`gemini`), next to the Claude logins.
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Config {
    // The user's Google AI Studio API key. Never sent to the page.
    pub(crate) key: String,
    // What cgemini gives Claude Code (ANTHROPIC_AUTH_TOKEN): only it opens /v1/*.
    pub(crate) token: String,
    pub(crate) model: String,
    pub(crate) fast_model: String,
    // The key's Gemini models that can generate content, from the check when saved.
    #[serde(default)]
    pub(crate) models: Vec<String>,
    pub(crate) saved_at: i64,
}

// At start: write cgemini.cmd again for this run's port, if a key is saved.
pub(crate) fn start(app: &Arc<App>) {
    script::start(app)
}
