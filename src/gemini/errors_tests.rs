// Google's error answers → the Anthropic status, error JSON and retry-after.

use super::*;

fn google(code: u16, status: &str, msg: &str, details: Value) -> String {
    json!({ "error": { "code": code, "message": msg, "status": status, "details": details } }).to_string()
}

fn kind(v: &Value) -> &str {
    v["error"]["type"].as_str().unwrap()
}

#[test]
fn statuses_map_to_anthropic_kinds() {
    for (from, to, k) in [
        (400, 400, "invalid_request_error"),
        (401, 401, "authentication_error"),
        (403, 403, "permission_error"),
        (404, 404, "not_found_error"),
        (500, 500, "api_error"),
        (503, 529, "overloaded_error"),
    ] {
        let (st, body, retry) = to_anthropic(from, &google(from, "X", "msg here", json!([])));
        assert_eq!((st, kind(&body), retry), (to, k, None), "{from}");
        assert_eq!(body["type"], "error");
        assert_eq!(body["error"]["message"], "msg here");
    }
}

#[test]
fn a_bad_key_is_an_authentication_error() {
    let details = json!([{ "@type": "type.googleapis.com/google.rpc.ErrorInfo", "reason": "API_KEY_INVALID" }]);
    let (st, body, _) = to_anthropic(400, &google(400, "INVALID_ARGUMENT", "API key not valid.", details));
    assert_eq!((st, kind(&body)), (401, "authentication_error"));
}

#[test]
fn rate_limits_carry_retry_after_in_whole_seconds() {
    let retry = |delay: &str| {
        let d = json!([{ "@type": "type.googleapis.com/google.rpc.RetryInfo", "retryDelay": delay }]);
        to_anthropic(429, &google(429, "RESOURCE_EXHAUSTED", "quota", d))
    };
    let (st, body, secs) = retry("13s");
    assert_eq!((st, kind(&body), secs), (429, "rate_limit_error", Some(13)));
    assert_eq!(retry("1.2s").2, Some(2));
    assert_eq!(to_anthropic(429, &google(429, "RESOURCE_EXHAUSTED", "quota", json!([]))).2, None);
}

#[test]
fn a_body_that_is_not_json_is_the_message() {
    let (st, body, _) = to_anthropic(502, "Bad Gateway");
    assert_eq!((st, kind(&body)), (502, "api_error"));
    assert_eq!(body["error"]["message"], "Bad Gateway");
    let (_, empty, _) = to_anthropic(500, "");
    assert!(!empty["error"]["message"].as_str().unwrap().is_empty());
}

#[test]
fn a_refused_schema_is_recognised() {
    let schema = google(400, "INVALID_ARGUMENT", "Invalid JSON payload received. Unknown name \"exclusiveMinimum\" at 'tools[0].function_declarations[1].parameters_json_schema'", json!([]));
    assert!(schema_refused(400, &schema));
    assert!(!schema_refused(400, &google(400, "INVALID_ARGUMENT", "contents is empty", json!([]))));
    assert!(!schema_refused(500, &schema));
}
