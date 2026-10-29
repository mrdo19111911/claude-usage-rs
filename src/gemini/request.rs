// An Anthropic Messages request → a Gemini GenerateContent body.

use crate::*;
use super::Sigs;

// The Gemini model for the model Claude Code asks for: a Gemini name as is; a Claude
// name with "haiku" → `fast`; any other → `main`.
pub(crate) fn pick_model(asked: &str, main: &str, fast: &str) -> String {
    let _ = (asked, main, fast);
    todo!()
}

// `legacy`: tool schemas in the older `parameters` form (the retry after Google refuses a
// schema), else `parametersJsonSchema`.
pub(crate) fn to_gemini(req: &Value, sigs: &Sigs, legacy: bool) -> Result<Value, String> {
    let _ = (req, sigs, legacy);
    todo!()
}

// /v1/messages/count_tokens: an estimate, characters / 4.
pub(crate) fn count_tokens(req: &Value) -> u64 {
    let _ = req;
    todo!()
}
