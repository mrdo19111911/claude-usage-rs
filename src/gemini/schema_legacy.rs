// Tool schemas in the older `parameters` form (Gemini's OpenAPI-style Schema), one node
// at a time: refs inlined, null types made `nullable`, only the keys it knows.

use crate::*;
use serde_json::Map;
use super::walk;

// What `parameters` (Gemini's Schema object) knows; everything else is dropped.
const KEEP: &[&str] = &["type", "format", "description", "nullable", "enum", "properties", "required", "items", "anyOf", "minItems", "maxItems", "minimum", "maximum", "minLength", "maxLength", "pattern"];
// Deeper than this is a loop the ref check missed, or a schema no model needs.
const MAX_DEPTH: usize = 40;

// The definition a `$ref` names ("#/$defs/X" → X), with the ref's own keys (a
// description, say) laid over it; two allOf lists are joined. A boolean schema (true:
// anything, false: nothing) has no keys: {} here, as `parameters` cannot say "nothing".
fn deref(node: &Map<String, Value>, defs: &Map<String, Value>) -> Option<(String, Value)> {
    let name = node.get("$ref")?.as_str()?.rsplit('/').next()?.to_string();
    let mut d = defs.get(&name)?.as_object().cloned().unwrap_or_default();
    for (k, v) in node.iter().filter(|(k, _)| *k != "$ref") {
        match (d.get_mut(k), v) {
            (Some(Value::Array(a)), Value::Array(more)) if k == "allOf" => a.extend(more.iter().cloned()),
            _ => {
                d.insert(k.clone(), v.clone());
            }
        }
    }
    Some((name, Value::Object(d)))
}

// One schema node → the whitelisted form. `chain`: the refs being inlined above.
pub(super) fn conv(s: &Value, defs: &Map<String, Value>, chain: &mut Vec<String>, depth: usize) -> Value {
    let Some(o) = s.as_object() else { return json!({ "type": "object" }) };
    if depth > MAX_DEPTH {
        return json!({ "type": "object" });
    }
    if o.contains_key("$ref") {
        return match deref(o, defs) {
            Some((name, d)) if !chain.contains(&name) => {
                chain.push(name);
                let out = conv(&d, defs, chain, depth + 1);
                chain.pop();
                out
            }
            _ => json!({ "type": "object" }),
        };
    }
    let mut o = o.clone();
    if let Some(Value::Array(all)) = o.remove("allOf") {
        merge_all_of(&mut o, &all, defs);
    }
    if let Some(c) = o.remove("const") {
        o.insert("enum".into(), json!([c]));
    }
    let mut out = Map::new();
    // type: [T, "null"] → T + nullable; several types → anyOf.
    match o.get("type") {
        Some(Value::Array(ts)) => {
            let real: Vec<&Value> = ts.iter().filter(|t| *t != "null").collect();
            if real.len() < ts.len() {
                out.insert("nullable".into(), json!(true));
            }
            match real.as_slice() {
                [one] => {
                    out.insert("type".into(), (*one).clone());
                }
                [] => {
                    out.insert("type".into(), json!("string"));
                }
                many => {
                    o.insert("anyOf".into(), many.iter().map(|t| json!({ "type": t })).collect());
                }
            }
        }
        Some(t) => {
            out.insert("type".into(), t.clone());
        }
        None => {}
    }
    let any = o.get("anyOf").or(o.get("oneOf")).and_then(Value::as_array).cloned();
    if let Some(any) = any {
        return any_of(&o, out, &any, defs, chain, depth);
    }
    for (k, v) in &o {
        if !KEEP.contains(&k.as_str()) || out.contains_key(k) {
            continue;
        }
        let v = match k.as_str() {
            "properties" => walk(k, v, &mut |p| conv(p, defs, chain, depth + 1)),
            "items" => conv(v.as_array().and_then(|a| a.first()).unwrap_or(v), defs, chain, depth + 1),
            _ => v.clone(),
        };
        out.insert(k.clone(), v);
    }
    tidy(&mut out);
    Value::Object(out)
}

// allOf → its members' properties and required laid into this node (LiteLLM drops it,
// which loses every property of a schema built that way).
fn merge_all_of(o: &mut Map<String, Value>, all: &[Value], defs: &Map<String, Value>) {
    for m in all {
        let m = m.as_object().map(|mo| deref(mo, defs).map(|(_, d)| d).unwrap_or_else(|| m.clone())).unwrap_or_default();
        let Some(m) = m.as_object() else { continue };
        for (k, v) in m {
            match (k.as_str(), o.get_mut(k)) {
                ("properties", Some(Value::Object(p))) => p.extend(v.as_object().cloned().unwrap_or_default()),
                ("required", Some(Value::Array(r))) => r.extend(v.as_array().cloned().unwrap_or_default()),
                (_, None) => {
                    o.insert(k.clone(), v.clone());
                }
                _ => {}
            }
        }
    }
}

// anyOf/oneOf: the null branch becomes `nullable`; one branch left is the node itself;
// several stay anyOf, alone on the node (Vertex refuses siblings, LiteLLM #11164). The
// node's own keys (properties, required, description, limits) go into each branch first,
// under the branch's own: a branch of only `required` keeps the tool's parameters.
fn any_of(o: &Map<String, Value>, mut out: Map<String, Value>, any: &[Value], defs: &Map<String, Value>, chain: &mut Vec<String>, depth: usize) -> Value {
    let real: Vec<&Value> = any.iter().filter(|b| b["type"] != "null").collect();
    let nullable = real.len() < any.len() || out.contains_key("nullable");
    let shared: Map<String, Value> = o.iter().filter(|(k, _)| !matches!(k.as_str(), "anyOf" | "oneOf")).map(|(k, v)| (k.clone(), v.clone())).collect();
    let mut branches: Vec<Value> = real.iter().map(|b| conv(&with_parent(b, &shared), defs, chain, depth + 1)).collect();
    for b in &mut branches {
        if nullable {
            b["nullable"] = json!(true);
        }
    }
    match branches.len() {
        0 => {
            out.insert("type".into(), json!("string"));
            Value::Object(out)
        }
        1 => branches.remove(0),
        _ => json!({ "anyOf": branches }),
    }
}

// A branch with its parent's keys as one more allOf member, so merge_all_of joins
// properties and required and keeps the branch's own keys first. A `$ref` branch keeps
// its ref, so conv still stops at a loop.
fn with_parent(b: &Value, shared: &Map<String, Value>) -> Value {
    let mut m = b.as_object().cloned().unwrap_or_default();
    if !shared.is_empty() {
        match m.get_mut("allOf") {
            Some(Value::Array(a)) => a.push(Value::Object(shared.clone())),
            _ => {
                m.insert("allOf".into(), json!([shared]));
            }
        }
    }
    Value::Object(m)
}

// What Gemini checks: a type always; enum and format only on strings; items on arrays;
// no empty properties; required only names that exist.
fn tidy(out: &mut Map<String, Value>) {
    if !out.contains_key("type") {
        let t = if out.contains_key("enum") { "string" } else { "object" };
        out.insert("type".into(), json!(t));
    }
    let string = out["type"] == "string";
    if !string {
        out.remove("enum");
        out.remove("format");
    }
    if string && !matches!(out.get("format").and_then(Value::as_str), None | Some("enum" | "date-time")) {
        out.remove("format");
    }
    if out["type"] == "array" && !out.contains_key("items") {
        out.insert("items".into(), json!({ "type": "object" }));
    }
    let names: Vec<String> = out.get("properties").and_then(Value::as_object).map(|p| p.keys().cloned().collect()).unwrap_or_default();
    if names.is_empty() {
        out.remove("properties");
    }
    if let Some(Value::Array(r)) = out.get_mut("required") {
        r.retain(|n| n.as_str().is_some_and(|n| names.iter().any(|k| k == n)));
        if r.is_empty() {
            out.remove("required");
        }
    }
}
