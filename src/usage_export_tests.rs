// The export (docs/specs/usage-log.md, "Export"): acc numbers, status-line shape, the
// download over HTTP, and usageLog in /api/state.

use super::*;
use crate::usage_log::tests::{account, temp_dir, test_app};
use crate::usage_log::{figures, FILE};

const T: i64 = 1_793_374_199_000; // 2026-10-30T... in ms

fn line(t: i64, id: &str, seed: bool) -> String {
    let mut v = json!({"t": t, "id": id, "plan": "Max 5x", "active": true,
        "five": {"u": 13, "resetsAt": 1_793_374_199_500i64}, "week": {"u": 54, "resetsAt": null},
        "scoped": [{"name": "Fable", "u": 40, "resetsAt": 1_793_851_199_000i64}]});
    if seed {
        v["seed"] = json!(true);
    }
    v.to_string()
}

#[test]
fn utc_times_end_in_z() {
    assert_eq!(utc(0), "1970-01-01T00:00:00Z");
    assert_eq!(utc(parse_iso("2026-10-30T12:48:12.999Z").unwrap()), "2026-10-30T12:48:12Z");
    assert_eq!(utc(parse_iso("2024-02-29T23:59:59Z").unwrap()), "2024-02-29T23:59:59Z");
    assert_eq!(file_name(parse_iso("2026-10-30T23:00:00Z").unwrap()), "scas-log-20261030.jsonl");
}

#[test]
fn accounts_are_numbered_by_their_order_and_unknown_ids_come_next() {
    let order = vec!["idA".to_string(), "idB".to_string()];
    let text = [line(T, "gone1", false), line(T + 1000, "idB", false), line(T + 2000, "idA", true), line(T + 3000, "gone2", false), line(T + 4000, "gone1", false)].join("\n");
    let out = export(&text, &order);
    let got: Vec<Value> = out.lines().map(|l| serde_json::from_str(l).unwrap()).collect();
    let names: Vec<&str> = got.iter().map(|v| v["account"].as_str().unwrap()).collect();
    assert_eq!(names, vec!["acc3", "acc2", "acc1", "acc4", "acc3"]);
    for bad in ["idA", "idB", "gone", "example.com"] {
        assert!(!out.contains(bad), "{bad} in the export");
    }
    assert_eq!(got[2]["seed"], true);
    assert_eq!(got[0], json!({"ts": utc(T), "account": "acc3", "plan": "Max 5x", "active": true,
        "five_hour": {"used_percentage": 13, "resets_at": 1_793_374_199},
        "seven_day": {"used_percentage": 54, "resets_at": null},
        "model_weekly": [{"name": "Fable", "used_percentage": 40, "resets_at": 1_793_851_199}],
        "seed": false, "source": "scas"}));
    assert!(got[0]["ts"].as_str().unwrap().ends_with('Z'));
}

#[test]
fn bad_lines_are_skipped() {
    let out = export(&format!("not json\n\n{}\n", line(T, "x", false)), &[]);
    assert_eq!(out.lines().count(), 1);
    assert!(out.ends_with('\n'));
}

#[test]
fn state_carries_usage_log_and_the_export_downloads() {
    let dir = temp_dir("http");
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let port = server.server_addr().to_ip().unwrap().port();
    let mut app = test_app(&dir);
    app.port = port;
    let app = Arc::new(app);
    {
        let mut st = app.st.lock().unwrap();
        st.accounts.push(account("idA", 13, 54, vec![]));
        app.start_usage_log(&mut st);
    }
    let a = app.st.lock().unwrap().accounts[0].clone();
    app.log_usage(figures(&a, true).unwrap());
    let srv = app.clone();
    thread::spawn(move || {
        for rq in server.incoming_requests() {
            handle(&srv, rq);
        }
    });
    let base = format!("http://127.0.0.1:{port}");
    let st: Value = ureq::get(&format!("{base}/api/state")).call().unwrap().into_json().unwrap();
    let bytes = fs::metadata(dir.join(FILE)).unwrap().len();
    assert_eq!(st["usageLog"]["lines"], 1);
    assert_eq!(st["usageLog"]["bytes"], bytes);
    assert!(st["usageLog"]["since"].is_i64());

    let r = ureq::get(&format!("{base}/api/usage-log/export")).call().unwrap();
    assert_eq!(r.header("Content-Type"), Some("application/x-ndjson"));
    let cd = r.header("Content-Disposition").unwrap().to_string();
    assert!(cd.starts_with("attachment; filename=\"scas-log-") && cd.ends_with(".jsonl\""), "{cd}");
    let body = r.into_string().unwrap();
    assert_eq!(body.lines().count(), 1);
    assert!(body.contains("\"account\":\"acc1\"") && !body.contains("idA") && !body.contains("example.com"));

    // Another site's name for 127.0.0.1 (DNS rebinding) gets nothing.
    let other = ureq::get(&format!("{base}/api/usage-log/export")).set("Host", "evil.test").call();
    assert!(matches!(other, Err(ureq::Error::Status(403, _))));
}
