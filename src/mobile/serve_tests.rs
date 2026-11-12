use super::*;

fn get(port: u16, path: &str, token: Option<&str>) -> (u16, String) {
    let mut rq = ureq::get(&format!("http://127.0.0.1:{port}{path}"));
    if let Some(t) = token {
        rq = rq.set("Authorization", &format!("Bearer {t}"));
    }
    match rq.call() {
        Ok(r) => (r.status(), r.into_string().unwrap()),
        Err(ureq::Error::Status(c, r)) => (c, r.into_string().unwrap_or_default()),
        Err(e) => panic!("{e}"),
    }
}

#[test]
fn serves_the_feed_with_the_token_only() {
    let state: StateFn = Box::new(|| r#"{"accounts":[{"id":"a","raw":{"x":1}}],"activeId":"a"}"#.to_string());
    let port = start(state, 0, "secret-token".into()).unwrap();
    let (code, body) = get(port, PATH, Some("secret-token"));
    assert_eq!(code, 200);
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!((v["v"].as_i64(), v["accounts"][0]["cli"].as_bool()), (Some(1), Some(true)));
    assert!(!body.contains("raw"));
    assert_eq!(get(port, PATH, None).0, 401);
    assert_eq!(get(port, PATH, Some("secret-tokeN")).0, 401);
    assert_eq!(get(port, "/api/state", Some("secret-token")).0, 404);
    let post = ureq::post(&format!("http://127.0.0.1:{port}{PATH}")).set("Authorization", "Bearer secret-token").call();
    assert!(matches!(post, Err(ureq::Error::Status(405, _))));
}
