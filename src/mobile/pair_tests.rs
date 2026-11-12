use super::*;

#[test]
fn token_is_43_base64url_chars_and_random() {
    let (a, b) = (new_token(), new_token());
    assert_eq!(a.len(), 43);
    assert!(a.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_'));
    assert_ne!(a, b);
}

#[test]
fn url_round_trips() {
    let u = url("192.168.1.20", 4778, "tok-_9");
    assert_eq!(u, "scas://pair?host=192.168.1.20&port=4778&token=tok-_9");
    let p = parse(&u).unwrap();
    assert_eq!((p.host.as_str(), p.port, p.token.as_str()), ("192.168.1.20", 4778, "tok-_9"));
}

#[test]
fn parse_refuses_what_is_not_a_pairing() {
    assert!(parse("https://pair?host=a&port=1&token=t").is_none());
    assert!(parse("scas://pair?host=a&port=1").is_none());
    assert!(parse("scas://pair?host=a&token=t").is_none());
    assert!(parse("scas://pair?host=&port=1&token=t").is_none());
    assert!(parse("scas://pair?host=a&port=99999&token=t").is_none());
}

#[test]
fn same_compares_whole_strings() {
    assert!(same("abc", "abc"));
    assert!(!same("abc", "abd"));
    assert!(!same("abc", "ab"));
    assert!(!same("", "a"));
}

#[test]
fn load_or_create_keeps_the_token() {
    let dir = std::env::temp_dir().join(format!("scas-pair-{}", new_token()));
    std::fs::create_dir_all(&dir).unwrap();
    let a = load_or_create(&dir).unwrap();
    assert_eq!(load_or_create(&dir).unwrap(), a);
    std::fs::remove_file(dir.join(TOKEN_FILE)).unwrap();
    assert_ne!(load_or_create(&dir).unwrap(), a);
    let _ = std::fs::remove_dir_all(&dir);
}
