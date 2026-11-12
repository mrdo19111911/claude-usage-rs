# Idle accounts: refresh their sign-in only when it is needed

Why: refresh tokens are single-use. Refreshing a parked account's token while another
Claude Code window still holds it means the same refresh token is used twice, which
looks like a stolen token (docs/specs/no-foreign-credentials.md). Before this change
every parked account was refreshed whenever its access token (about 6 h) ran out:
tens of refreshes a day for nothing.

## Rule (`src/idle_refresh.rs`, tests `src/idle_refresh_tests.rs`)

- `needed(accounts, t)`: true when an account in use is near a limit:
  `nearness(usage_now) >= NEED_FROM` (80 five-hour points; the week counts as in
  check-schedule.md), or either window of it is full. Then the switch's choice needs
  fresh figures for the others.
- `may_refresh(need, force, left_at, t)`: a parked account's sign-in may be refreshed
  when `need` or `force` (Check now / Refresh now), and not within `LEFT_GUARD_MS`
  (15 min) of it stopping being in use (`left_at`), when an old window may still hold it.
- `mark_in_use` sets `left_at = t` when an account goes from in use to not in use.
- The live Claude Code login is never refreshed here (unchanged).

## In the poll (`poll.rs`)

- A parked account whose token is still valid is read as before (no refresh needed).
- A parked account whose token has expired (or gets a 401) and may not be refreshed is
  **waiting**: no request, no error shown, its last figures stand, `next_check` is
  `SLOWEST_MS` away and `waiting = true`.
- When `needed`, every waiting account is checked at once, not at its `next_check`.
- `switch` (verify_login) still refreshes the account it switches to: Claude Code
  needs a live sign-in.

## Done when

- Tests: `needed` (far, near 5-hour, near week, full, nobody in use); `may_refresh`
  (no need, need, force, inside and after the guard); `mark_in_use` sets `left_at`.
- All tests green; a parked account with an expired token shows no error on the page.
