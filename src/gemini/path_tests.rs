// The user's PATH value: one folder added once, removed again, the rest kept as it was.

use super::path::{path_with, path_without};

const BIN: &str = r"C:\Users\me\.claude-usage-dashboard\bin";

#[test]
fn adds_at_the_end() {
    assert_eq!(path_with(r"C:\a;%USERPROFILE%\b", BIN), format!(r"C:\a;%USERPROFILE%\b;{BIN}"));
    assert_eq!(path_with("", BIN), BIN);
    assert_eq!(path_with(r"C:\a;", BIN), format!(r"C:\a;{BIN}"), "no empty entry");
}

#[test]
fn adds_only_once_in_any_case_or_trailing_slash() {
    let old = format!(r"C:\a;{BIN};C:\b");
    assert_eq!(path_with(&old, BIN), old);
    let upper = format!(r"C:\a;{}\", BIN.to_uppercase());
    assert_eq!(path_with(&upper, BIN), upper);
    // A longer folder of the same start is another folder.
    let other = format!(r"{BIN}-old");
    assert_eq!(path_with(&other, BIN), format!("{other};{BIN}"));
}

#[test]
fn removes_every_copy_and_nothing_else() {
    assert_eq!(path_without(&format!(r"C:\a;{BIN};C:\b"), BIN), r"C:\a;C:\b");
    assert_eq!(path_without(&format!(r"{};C:\a;{BIN}\", BIN.to_lowercase()), BIN), r"C:\a");
    assert_eq!(path_without(BIN, BIN), "");
    let other = format!(r"C:\a;;{BIN}-old");
    assert_eq!(path_without(&other, BIN), other, "untouched when absent");
}
