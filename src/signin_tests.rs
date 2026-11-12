// Tests of the sign-in wording (docs/REVIEW_DE_HIEU.md section 5): plain words for office
// staff, no raw server text, and a page title that agrees with its message.

use super::*;

#[test]
fn a_refused_sign_in_says_what_to_do() {
    assert_eq!(SIGN_IN_REFUSED, "Anthropic did not accept the sign-in. Close this tab and click + Add account again.");
}

#[test]
fn added_lines_are_short_and_say_sign_in() {
    assert_eq!(added_in_browser("a@x"), "Added a@x");
    assert_eq!(added_from_code("a@x"), "Added a@x (new Claude Code sign-in)");
}

#[test]
fn the_page_title_agrees_with_the_message() {
    let title = |ok, msg| {
        let page = login_page(ok, msg, 4777);
        page.split("<title>").nth(1).and_then(|t| t.split("</title>").next()).unwrap().to_string()
    };
    assert_eq!(title(true, "Added a@x"), "Account added");
    assert_eq!(title(true, "Signed in again to a@x"), "Signed in");
    assert_eq!(title(true, "a@x is the account Claude Code uses now; it is here already."), "Signed in");
    assert_eq!(title(false, SIGN_IN_REFUSED), "Sign-in not completed");
}
