// Tool input schemas (JSON Schema) as Gemini takes them. `clean` for
// `parametersJsonSchema` (real JSON Schema, CCR sends it whole); `legacy` for the older
// OpenAPI-style `parameters`, ported from LiteLLM's build_vertex_schema (unpack_defs,
// anyOf+null → nullable, enum only on strings, items filled in, a key whitelist).

use crate::*;
use serde_json::Map;

// Keys whose value is one schema (or a list of them), a list of schemas, or a map of
// name → schema. Any other value (enum, default, examples...) is data, never walked.
const ONE: &[&str] = &["items", "additionalProperties", "not", "if", "then", "else", "contains", "propertyNames", "additionalItems", "unevaluatedItems", "unevaluatedProperties"];
const LIST: &[&str] = &["anyOf", "oneOf", "allOf", "prefixItems"];
const MAP: &[&str] = &["properties", "$defs", "definitions", "patternProperties", "dependentSchemas"];

// For `parametersJsonSchema`: drops $schema, $id, $comment, examples; const → enum.
pub(crate) fn clean(s: &Value) -> Value {
    let Some(o) = s.as_object() else { return s.clone() };
    let mut out = Map::new();
    for (k, v) in o {
        match k.as_str() {
            "$schema" | "$id" | "$comment" | "examples" => {}
            "const" => {
                out.insert("enum".into(), json!([v]));
            }
            _ => {
                out.insert(k.clone(), walk(k, v, &mut |s| clean(s)));
            }
        }
    }
    Value::Object(out)
}

// The value of key `k` with `f` applied to the schemas in it.
fn walk(k: &str, v: &Value, f: &mut dyn FnMut(&Value) -> Value) -> Value {
    match v {
        Value::Object(m) if MAP.contains(&k) => Value::Object(m.iter().map(|(n, s)| (n.clone(), f(s))).collect()),
        Value::Array(a) if ONE.contains(&k) || LIST.contains(&k) => Value::Array(a.iter().map(&mut *f).collect()),
        Value::Object(_) if ONE.contains(&k) => f(v),
        _ => v.clone(),
    }
}

// For the older `parameters`: only the keys it knows (docs/SO_SANH_GEMINI_PROXY.md, Chọn 2).
pub(crate) fn legacy(s: &Value) -> Value {
    let mut defs = Map::new();
    for k in ["definitions", "$defs"] {
        if let Some(d) = s[k].as_object() {
            defs.extend(d.clone());
        }
    }
    let mut out = old::conv(s, &defs, &mut vec![], 0);
    if out.get("type").is_none() && out.get("anyOf").is_none() {
        out["type"] = json!("object");
    }
    out
}

// The `parameters` form, node by node.
#[path = "schema_legacy.rs"]
mod old;
