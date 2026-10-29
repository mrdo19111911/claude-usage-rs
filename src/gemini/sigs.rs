// Gemini 3's thought signatures, kept by the tool_use id they came with: Claude Code
// sends back only id, name and input, and Gemini refuses a function call without one.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

#[derive(Default)]
pub(crate) struct Sigs(Mutex<HashMap<String, (String, Instant)>>);

impl Sigs {
    pub(crate) fn put(&self, id: &str, sig: &str) {
        let _ = (id, sig);
        todo!()
    }
    pub(crate) fn get(&self, id: &str) -> Option<String> {
        let _ = id;
        todo!()
    }
}
