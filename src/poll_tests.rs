// Tests for the Desktop-token usage fallback decision (docs/specs/desktop-token-fallback.md).

use super::wants_desktop_fallback;
use crate::ApiErr;

#[test]
fn auth_failure_falls_back_to_desktop_token() {
    // A parked account whose stored refresh token was spent by Claude Desktop (rotation).
    assert!(wants_desktop_fallback(
        &ApiErr::Auth("refresh failed (400): invalid — /login this account again".into()),
        false,
    ));
    // Still true even if it happened to be the live login.
    assert!(wants_desktop_fallback(&ApiErr::Auth("usage HTTP 401".into()), true));
}

#[test]
fn live_expired_falls_back_only_when_live() {
    let m = "token expired: Claude Code renews it the next time it runs";
    assert!(wants_desktop_fallback(&ApiErr::Other(m.into()), true));
    // A parked account with that message must not trigger the live-only path.
    assert!(!wants_desktop_fallback(&ApiErr::Other(m.into()), false));
}

#[test]
fn rate_limit_never_falls_back() {
    assert!(!wants_desktop_fallback(&ApiErr::Slow(Some(1000)), true));
    assert!(!wants_desktop_fallback(&ApiErr::Slow(None), false));
}

#[test]
fn other_errors_do_not_fall_back() {
    assert!(!wants_desktop_fallback(&ApiErr::Other("usage HTTP 500".into()), false));
    assert!(!wants_desktop_fallback(&ApiErr::Other("network error".into()), true));
}
