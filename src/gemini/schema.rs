// Tool input schemas (JSON Schema) as Gemini takes them.

use crate::*;

// For `parametersJsonSchema`: drops $schema, $id, $comment, examples; const → enum.
pub(crate) fn clean(s: &Value) -> Value {
    let _ = s;
    todo!()
}

// For the older `parameters`: only the keys it knows (docs/SO_SANH_GEMINI_PROXY.md, Chọn 2).
pub(crate) fn legacy(s: &Value) -> Value {
    let _ = s;
    todo!()
}
