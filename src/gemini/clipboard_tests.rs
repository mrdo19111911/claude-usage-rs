// The clipboard's text as Windows hands it over (UTF-16, ending at a NUL).

use super::clipboard::{text_of, MAX_UNITS};

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

#[test]
fn text_ends_at_the_first_nul() {
    let mut w = wide("hello");
    w.extend([0, 65, 66]); // GlobalSize can be more than the text: what follows is not read
    assert_eq!(text_of(&w).as_deref(), Some("hello"));
    assert_eq!(text_of(&wide("tiếng Việt")).as_deref(), Some("tiếng Việt"), "no NUL at all");
    assert_eq!(text_of(&[]).as_deref(), Some(""));
}

#[test]
fn long_or_broken_text_is_not_copied_out() {
    assert_eq!(text_of(&vec![65; MAX_UNITS]).map(|s| s.len()), Some(MAX_UNITS));
    assert_eq!(text_of(&vec![65; MAX_UNITS + 1]), None, "cannot be a key");
    assert_eq!(text_of(&[0xD800, 65]), None, "a lone surrogate");
}

// By hand only (it reads the real clipboard): Set-Clipboard 'hello' in PowerShell first,
// then cargo test clipboard_reads_hello -- --ignored
#[cfg(windows)]
#[test]
#[ignore]
fn clipboard_reads_hello() {
    use super::watch::{Read, Source};
    let c = super::clipboard::System;
    assert_ne!(c.seq(), 0, "a sequence number");
    match c.read() {
        Read::Text(t) => assert_eq!(t.trim(), "hello"),
        other => panic!("{other:?}"),
    }
}
