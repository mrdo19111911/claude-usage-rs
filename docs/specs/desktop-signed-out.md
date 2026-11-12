# Spec: a saved Claude Desktop sign-in that has expired

## Why
2026-11-12 10:17:51: a switch to thuc.lesy (fb2bda4d) opened Claude Desktop on its sign-in
screen (main.log: `sign_in_cookie_present: false`). Its saved profile has had no claude.ai
`sessionKey` cookie since at least 03:29 that night: the server ended that session. The app
still offered the account as "Use here" and auto-switch could pick it.

## What
- `desktop::cookie_session::session_state(cookies, now_ms) -> Option<bool>` (module
  `src/desktop/cookie_session.rs`, tests `cookie_session_tests.rs`):
  - reads a Chromium cookie store (SQLite, table `cookies`) from a temporary copy, so the
    file and its journal are never touched;
  - `Some(true)`: a row `name = 'sessionKey'` whose `host_key` is `claude.ai` or ends in
    `.claude.ai` and that has not expired (`expires_utc = 0`, a session cookie, or later
    than now; Chromium time is microseconds since 1601-01-01 UTC);
  - `Some(false)`: the store reads and holds no such row;
  - `None`: no file, it does not read as a cookie store, or it is locked (unknown: never
    blocks anything). A running Claude Desktop locks its own store, so it reads as `None`
    until the app has closed; a switch reads it only after closing the app.
- `Store::sign_in_expired(account, now)`: its profile's state is `Some(false)`.
- An expired account:
  - is skipped by Desktop auto-switch (as one with no saved sign-in);
  - a switch to it by hand is refused with "Its saved sign-in has expired: use Sign in
    again" (the same path as "Use here (first time)": Desktop opens signed out and the user
    picks the account);
  - `/api/state` gives `signInExpired: true`; its card button reads "Sign in again" and the
    "In use now" list says "sign-in expired: sign in again".
- Leaving an account: when Desktop's own cookies hold no live `sessionKey` (`Some(false)`)
  but the saved profile does (`Some(true)`), the saved profile is kept, not overwritten
  with a signed-out copy.

## Until when a saved sign-in lasts (asked 2026-11-12)
claude.ai sets `sessionKey` for 28 days and sets it again, for 28 more, when the account is
used (0080af61: issued 10-28, renewed 11-11 15:48, ends 12-09). A saved sign-in that is not
used before then ends. Separately, Log out ends the session at once for every saved copy
(fb2bda4d: saved 10-28 16:32, logged out 17:21, refused 11-12 03:18).
- `cookie_session::read_session(cookies, now_ms) -> Option<Session>`: `Session::Live(until)`
  with `until` the latest live `sessionKey` expiry in Unix ms (None: a cookie without
  expiry), or `Session::Ended`; None as for `session_state`, which it now backs.
- `Store::saved_session(account, now)`: that for its saved profile (one read per account
  per `/api/state`; `sign_in_expired` is `Some(Session::Ended)`).
- `/api/state` gives each Desktop account `signInUntil` (Unix ms or null; null while it is
  the one signed in, as the app renews its own).
- Its card's Claude Desktop login shows "Sign-in until Dec 10 · 27 days" under its
  auto-switch, amber from 7 days left, with a tooltip saying each use renews it for 28
  days and that Log out ends it at once.

## Done when
- Tests: session_state on a store with a live / expired / session-only / other-host
  sessionKey, an empty store, a missing file and a non-SQLite file; pick_desktop skips an
  expired account; the save guard keeps a live profile.
- On this computer: fb2bda4d reports expired, the other three do not.
  Measured 2026-11-12 (`cargo test real_profiles -- --ignored --nocapture`): fb2bda4d
  `Some(false)`; 0080af61, 359a6468, 539a23b5, 75910745, d8828315 `Some(true)`; about 2 ms
  each; the running app's store `None` (locked).

Prior art: docs/SO_SANH_DESKTOP_SIGNED_OUT.md.
