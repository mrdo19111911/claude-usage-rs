// /api/gemini/* for the dashboard page: save the key, pick the model, remove.

use crate::*;

// Some(answer) when `path` is one of these, else None. Same answer shape as server.rs.
pub(crate) fn api(app: &Arc<App>, method: &str, path: &str, body: &Value) -> Option<Result<(u16, String, &'static str), String>> {
    let _ = (app, method, path, body);
    None
}

// What /api/state shows of it (never the key): null, or {keyEnd, model, fastModel,
// models, command}.
pub(crate) fn public(st: &State) -> Value {
    let _ = st;
    Value::Null
}
