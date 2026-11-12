// public_state's JSON → the phone's feed, version 1 (docs/specs/mobile-feed.md).
// Only the fields listed in the spec are copied; anything new in public_state stays out.
use serde_json::{json, Value};

pub(crate) const VERSION: i64 = 1;

pub(crate) fn from_public(s: &Value) -> Value {
    let active = s["activeId"].as_str();
    // The uuid Claude Desktop is signed in to now, if any.
    let desk = s["desktop"]["accounts"]
        .as_array()
        .and_then(|d| d.iter().find(|r| r["signedIn"].as_bool() == Some(true)))
        .and_then(|r| r["uuid"].as_str());
    let accounts: Vec<Value> = s["accounts"]
        .as_array()
        .map(|a| a.iter().map(|a| account(a, active, desk)).collect())
        .unwrap_or_default();
    let set = &s["settings"];
    json!({
        "v": VERSION,
        "version": s["version"], "now": s["now"], "lastPollAt": s["lastPollAt"], "polling": s["polling"],
        "autoSwitch": set["autoSwitch"], "threshold": set["threshold"], "weekThreshold": set["weekThreshold"],
        "accounts": accounts,
    })
}

fn account(a: &Value, active: Option<&str>, desk: Option<&str>) -> Value {
    let id = a["id"].as_str();
    let uuid = a["uuid"].as_str();
    json!({
        "id": a["id"], "email": a["email"], "plan": a["plan"], "enabled": a["enabled"],
        "cli": id.is_some() && id == active,
        "desktop": uuid.is_some() && uuid == desk,
        "error": a["error"],
        "usage": usage(&a["usage"]),
    })
}

// Usage as public_state has it (model::Usage), field by field.
fn usage(u: &Value) -> Value {
    if !u.is_object() {
        return Value::Null;
    }
    let win = |w: &Value| json!({"u": w["u"], "resetsAt": w["resetsAt"]});
    let scoped: Vec<Value> = u["scoped"]
        .as_array()
        .map(|s| s.iter().map(|x| json!({"name": x["name"], "u": x["u"], "resetsAt": x["resetsAt"]})).collect())
        .unwrap_or_default();
    json!({"five": win(&u["five"]), "week": win(&u["week"]), "fetchedAt": u["fetchedAt"], "scoped": scoped})
}

#[cfg(test)]
#[path = "feed_tests.rs"]
mod tests;
