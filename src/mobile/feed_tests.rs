use super::*;
use serde_json::json;

fn public() -> Value {
    json!({
        "now": 5, "lastPollAt": 4, "polling": false, "version": "1.6.1", "activeId": "b",
        "settings": {"threshold": 95, "weekThreshold": 99, "autoSwitch": true, "pollSeconds": 0},
        "events": [{"t": 1, "msg": "x"}], "loginFile": "C:\\secret\\path", "claudeJson": "p",
        "desktop": {"supported": true, "found": true, "dir": "d", "accounts": [
            {"uuid": "ua", "signedIn": true}, {"uuid": "ub", "signedIn": false}]},
        "accounts": [
            {"id": "a", "uuid": "ua", "email": "a@x", "plan": "Max", "enabled": true, "nextCheck": 9,
             "usage": {"five": {"u": 9, "resetsAt": 100}, "week": {"u": 24, "resetsAt": null}, "fetchedAt": 3,
                       "scoped": [{"name": "Fable", "u": 3, "resetsAt": 100}]},
             "error": null, "raw": {"five_hour": {}}, "history": [[1, 2, 3]]},
            {"id": "b", "uuid": "ub", "email": "b@x", "plan": "Pro", "enabled": false,
             "usage": null, "error": "token expired", "raw": null, "history": []}
        ]
    })
}

#[test]
fn keeps_only_the_phone_fields() {
    let f = from_public(&public());
    assert_eq!(f["v"], 1);
    assert_eq!((f["version"].as_str(), f["now"].as_i64(), f["lastPollAt"].as_i64()), (Some("1.6.1"), Some(5), Some(4)));
    assert_eq!((f["threshold"].as_i64(), f["weekThreshold"].as_i64(), f["autoSwitch"].as_bool()), (Some(95), Some(99), Some(true)));
    let text = f.to_string();
    for gone in ["raw", "history", "uuid", "events", "loginFile", "claudeJson", "secret", "nextCheck", "dir"] {
        assert!(!text.contains(gone), "{gone} leaked: {text}");
    }
}

#[test]
fn marks_cli_and_desktop_and_keeps_order() {
    let f = from_public(&public());
    let a = f["accounts"].as_array().unwrap();
    assert_eq!(a.len(), 2);
    assert_eq!((a[0]["id"].as_str(), a[1]["id"].as_str()), (Some("a"), Some("b")));
    assert_eq!((a[0]["cli"].as_bool(), a[0]["desktop"].as_bool()), (Some(false), Some(true)));
    assert_eq!((a[1]["cli"].as_bool(), a[1]["desktop"].as_bool()), (Some(true), Some(false)));
}

#[test]
fn copies_usage_and_errors() {
    let f = from_public(&public());
    let a = &f["accounts"];
    assert_eq!(a[0]["usage"]["five"]["u"], 9);
    assert_eq!(a[0]["usage"]["week"]["resetsAt"], Value::Null);
    assert_eq!(a[0]["usage"]["scoped"][0]["name"], "Fable");
    assert_eq!((a[0]["email"].as_str(), a[0]["plan"].as_str(), a[0]["enabled"].as_bool()), (Some("a@x"), Some("Max"), Some(true)));
    assert_eq!(a[1]["usage"], Value::Null);
    assert_eq!(a[1]["error"], "token expired");
}

#[test]
fn copes_with_an_empty_state() {
    let f = from_public(&json!({}));
    assert_eq!(f["accounts"], json!([]));
    assert_eq!(f["v"], 1);
}
