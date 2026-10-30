// The usage log's export: the same lines in the shape of Claude Code's status line, the
// accounts named acc1, acc2… (docs/specs/usage-log.md, "Export").

use crate::*;
use std::collections::HashMap;

// GET /api/usage-log/export: a download of the whole log (server.rs checked the Host).
pub(crate) fn respond(app: &App, rq: tiny_http::Request) {
    let text = app.usage_log.path().and_then(|p| fs::read_to_string(p).ok()).unwrap_or_default();
    let order: Vec<String> = app.st.lock().unwrap().accounts.iter().map(|a| a.id.clone()).collect();
    let disposition = format!("attachment; filename=\"{}\"", file_name(now()));
    let resp = tiny_http::Response::from_string(export(&text, &order))
        .with_header(tiny_http::Header::from_bytes("Content-Type", "application/x-ndjson").unwrap())
        .with_header(tiny_http::Header::from_bytes("Content-Disposition", disposition).unwrap())
        .with_header(tiny_http::Header::from_bytes("Cache-Control", "no-store").unwrap());
    let _ = rq.respond(resp);
}

// One line out per log line. `order`: state.json's account ids, acc1 first; ids no
// longer there get the next numbers as they first appear. Unreadable lines are skipped.
pub(crate) fn export(text: &str, order: &[String]) -> String {
    let mut names: HashMap<String, usize> = order.iter().enumerate().map(|(i, id)| (id.clone(), i + 1)).collect();
    let mut out = String::new();
    for v in text.lines().filter_map(|l| serde_json::from_str::<Value>(l).ok()) {
        let Some(t) = v["t"].as_i64() else { continue };
        let id = v["id"].as_str().unwrap_or_default().to_string();
        let next = names.len() + 1;
        let n = *names.entry(id).or_insert(next);
        out.push_str(&export_line(&v, t, n).to_string());
        out.push('\n');
    }
    out
}

fn export_line(v: &Value, t: i64, n: usize) -> Value {
    let win = |w: &Value| json!({ "used_percentage": w["u"], "resets_at": secs(&w["resetsAt"]) });
    let scoped: Vec<Value> = v["scoped"].as_array().map(Vec::as_slice).unwrap_or_default().iter()
        .map(|s| json!({ "name": s["name"], "used_percentage": s["u"], "resets_at": secs(&s["resetsAt"]) }))
        .collect();
    json!({
        "ts": utc(t), "account": format!("acc{n}"), "plan": v["plan"], "active": v["active"].as_bool().unwrap_or(false),
        "five_hour": win(&v["five"]), "seven_day": win(&v["week"]), "model_weekly": scoped,
        "seed": v["seed"].as_bool().unwrap_or(false), "source": "claude-usage",
    })
}

// ms -> Unix seconds, as the status line's resets_at; null stays null.
fn secs(ms: &Value) -> Value {
    ms.as_i64().map_or(Value::Null, |ms| json!(ms.div_euclid(1000)))
}

// ms -> "2026-10-30T12:48:12Z" (days to a civil date: H. Hinnant's algorithm).
pub(crate) fn utc(ms: i64) -> String {
    let (s, days) = (ms.div_euclid(1000), ms.div_euclid(86_400_000));
    let (y, m, d) = civil(days);
    let sod = s - days * 86_400;
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", sod / 3600, sod / 60 % 60, sod % 60)
}

// "claude-usage-log-<yyyymmdd>.jsonl", the UTC date of `ms`.
pub(crate) fn file_name(ms: i64) -> String {
    let (y, m, d) = civil(ms.div_euclid(86_400_000));
    format!("claude-usage-log-{y:04}{m:02}{d:02}.jsonl")
}

fn civil(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}
